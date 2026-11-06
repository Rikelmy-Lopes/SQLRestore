//! PostgreSQL restore adapter.

use std::fs;
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

use postgres::{CancelToken, Client, NoTls};

use crate::domain::cancellation::Cancellation;
use crate::domain::error::RestoreError;
use crate::domain::restore::{
    RestoreProgress, RestoreReport, RestoreRequest, is_protected_database,
};
use crate::domain::sql_script::{ScriptChunk, parse_sql_script};
use crate::ports::database_restorer::DatabaseRestorer;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAINTENANCE_DATABASE: &str = "postgres";

/// Restores plain SQL backups through a PostgreSQL connection.
#[derive(Default)]
pub struct PostgresRestorer {
    cancel_token: Mutex<Option<CancelToken>>,
}

impl PostgresRestorer {
    pub fn new() -> Self {
        Self::default()
    }

    fn connect(request: &RestoreRequest, database: &str) -> Result<Client, RestoreError> {
        Self::client_config(request, database)
            .connect(NoTls)
            .map_err(|error| RestoreError::Connection(error.to_string()))
    }

    fn client_config(request: &RestoreRequest, database: &str) -> postgres::Config {
        let mut config = postgres::Config::new();
        config
            .host(&request.connection.host)
            .port(request.connection.port)
            .user(&request.connection.user)
            .dbname(database)
            .application_name("SQLRestore")
            .connect_timeout(CONNECT_TIMEOUT);
        if !request.password.is_empty() {
            config.password(&request.password);
        }
        config
    }

    fn recreate_database(request: &RestoreRequest) -> Result<(), RestoreError> {
        let database = request.connection.database.trim();
        if is_protected_database(database) {
            return Err(RestoreError::InvalidRequest(format!(
                "refusing to delete the maintenance database \"{database}\""
            )));
        }

        let mut client = Self::connect(request, MAINTENANCE_DATABASE).map_err(|error| {
            RestoreError::DatabaseRecreate(format!(
                "could not connect to the maintenance database \"{MAINTENANCE_DATABASE}\": {error}"
            ))
        })?;

        client
            .execute(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
                 WHERE datname = $1 AND pid <> pg_backend_pid()",
                &[&database],
            )
            .map_err(|error| recreate_error("could not terminate open connections", &error))?;

        let quoted = quote_identifier(database);
        client
            .batch_execute(&format!("DROP DATABASE IF EXISTS {quoted}"))
            .map_err(|error| recreate_error("could not drop the database", &error))?;
        client
            .batch_execute(&format!("CREATE DATABASE {quoted}"))
            .map_err(|error| recreate_error("could not create the database", &error))?;

        Ok(())
    }

    fn execute_chunk(client: &mut Client, chunk: &ScriptChunk) -> Result<(), RestoreError> {
        match &chunk.copy_data {
            Some(data) => {
                let query = chunk.sql.trim_end().trim_end_matches(';').trim_end();
                let mut writer = client
                    .copy_in(query)
                    .map_err(|error| database_error(chunk, error))?;
                writer
                    .write_all(data.as_bytes())
                    .map_err(|error| database_io_error(chunk, error))?;
                writer
                    .finish()
                    .map(|_| ())
                    .map_err(|error| database_error(chunk, error))
            }
            None => client
                .batch_execute(&chunk.sql)
                .map_err(|error| database_error(chunk, error)),
        }
    }
}

impl DatabaseRestorer for PostgresRestorer {
    fn restore(
        &self,
        request: &RestoreRequest,
        cancellation: &Cancellation,
        progress: &dyn Fn(RestoreProgress),
    ) -> Result<RestoreReport, RestoreError> {
        request.validate()?;
        let contents = fs::read_to_string(&request.file_path)?;
        let total_bytes = contents.len() as u64;
        let chunks = parse_sql_script(&contents);

        if cancellation.is_cancelled() {
            return Err(RestoreError::Cancelled);
        }

        if request.recreate_database {
            Self::recreate_database(request)?;
        }

        let mut client = Self::connect(request, &request.connection.database)?;
        {
            let mut token = self
                .cancel_token
                .lock()
                .map_err(|_| RestoreError::Cancel("cancel state is poisoned".to_string()))?;
            *token = Some(client.cancel_token());
        }

        let outcome = run_chunks(&mut client, &chunks, total_bytes, cancellation, progress);

        if let Ok(mut token) = self.cancel_token.lock() {
            *token = None;
        }

        outcome
    }

    fn cancel(&self) -> Result<(), RestoreError> {
        let token = self
            .cancel_token
            .lock()
            .map_err(|_| RestoreError::Cancel("cancel state is poisoned".to_string()))?;
        if let Some(token) = token.as_ref() {
            token
                .cancel_query(NoTls)
                .map_err(|error| RestoreError::Cancel(error.to_string()))?;
        }
        Ok(())
    }
}

fn run_chunks(
    client: &mut Client,
    chunks: &[ScriptChunk],
    total_bytes: u64,
    cancellation: &Cancellation,
    progress: &dyn Fn(RestoreProgress),
) -> Result<RestoreReport, RestoreError> {
    let mut statements = 0usize;

    for chunk in chunks {
        if cancellation.is_cancelled() {
            return Err(RestoreError::Cancelled);
        }
        PostgresRestorer::execute_chunk(client, chunk)?;
        statements += 1;
        progress(RestoreProgress {
            processed_bytes: chunk.end_offset as u64,
            total_bytes,
        });
    }

    progress(RestoreProgress {
        processed_bytes: total_bytes,
        total_bytes,
    });

    Ok(RestoreReport {
        statements,
        bytes: total_bytes,
        elapsed: Duration::ZERO,
    })
}

fn database_error(chunk: &ScriptChunk, error: postgres::Error) -> RestoreError {
    RestoreError::Database {
        line: statement_line(chunk, &error),
        message: describe_database_error(&error),
    }
}

fn database_io_error(chunk: &ScriptChunk, error: std::io::Error) -> RestoreError {
    RestoreError::Database {
        line: chunk.start_line,
        message: format!("could not send COPY data: {error}"),
    }
}

fn recreate_error(action: &str, error: &postgres::Error) -> RestoreError {
    RestoreError::DatabaseRecreate(format!("{action}: {}", describe_database_error(error)))
}

/// Quotes a PostgreSQL identifier.
fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Full error text, including the server fields hidden by `Display`.
fn describe_database_error(error: &postgres::Error) -> String {
    let Some(database_error) = error.as_db_error() else {
        return error.to_string();
    };

    let mut parts = vec![format!(
        "{}: {}",
        database_error.severity(),
        database_error.message()
    )];
    if let Some(detail) = database_error.detail() {
        parts.push(format!("DETAIL: {detail}"));
    }
    if let Some(hint) = database_error.hint() {
        parts.push(format!("HINT: {hint}"));
    }
    if let Some(context) = database_error.where_() {
        parts.push(format!("WHERE: {context}"));
    }
    if let Some(schema) = database_error.schema() {
        parts.push(format!("SCHEMA: {schema}"));
    }
    if let Some(table) = database_error.table() {
        parts.push(format!("TABLE: {table}"));
    }
    if let Some(column) = database_error.column() {
        parts.push(format!("COLUMN: {column}"));
    }
    if let Some(datatype) = database_error.datatype() {
        parts.push(format!("DATATYPE: {datatype}"));
    }
    if let Some(constraint) = database_error.constraint() {
        parts.push(format!("CONSTRAINT: {constraint}"));
    }
    parts.push(format!("SQLSTATE: {}", database_error.code().code()));

    parts.join(" | ")
}

/// Line in the source file where the failing statement position is.
fn statement_line(chunk: &ScriptChunk, error: &postgres::Error) -> usize {
    let position = error
        .as_db_error()
        .and_then(|database_error| database_error.position())
        .and_then(|position| match position {
            postgres::error::ErrorPosition::Original(position) => Some(*position as usize),
            postgres::error::ErrorPosition::Internal { .. } => None,
        });

    chunk.start_line + position.map_or(0, |position| newlines_before(&chunk.sql, position))
}

/// Counts line breaks before a 1-based character position.
fn newlines_before(sql: &str, position: usize) -> usize {
    sql.chars()
        .take(position.saturating_sub(1))
        .filter(|character| *character == '\n')
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_restorer_has_no_cancel_token() {
        let restorer = PostgresRestorer::new();
        assert!(restorer.cancel().is_ok());
    }

    #[test]
    fn invalid_request_is_rejected_before_connecting() {
        let restorer = PostgresRestorer::new();
        let request = RestoreRequest {
            connection: crate::domain::config::ConnectionSettings::default(),
            password: String::new(),
            file_path: "missing.sql".into(),
            recreate_database: false,
        };
        let result = restorer.restore(&request, &Cancellation::new(), &|_progress| {});
        assert!(matches!(result, Err(RestoreError::InvalidRequest(_))));
    }

    #[test]
    fn counts_line_breaks_before_error_position() {
        let sql = "SELECT 1;\nSELECT\n  2;";
        assert_eq!(newlines_before(sql, 1), 0);
        assert_eq!(newlines_before(sql, 10), 0);
        assert_eq!(newlines_before(sql, 11), 1);
        assert_eq!(newlines_before(sql, 18), 2);
    }

    #[test]
    fn counts_multibyte_characters_as_one_position() {
        assert_eq!(newlines_before("é\nx", 3), 1);
    }

    #[test]
    fn quotes_database_identifiers() {
        assert_eq!(quote_identifier("shop"), "\"shop\"");
        assert_eq!(quote_identifier("my\"db"), "\"my\"\"db\"");
    }

    #[test]
    fn recreate_rejects_maintenance_database_without_connecting() {
        let restorer = PostgresRestorer::new();
        let (_file, path) = tempfile::NamedTempFile::new()
            .expect("temporary file")
            .keep()
            .expect("keep temporary file");
        let request = RestoreRequest {
            connection: crate::domain::config::ConnectionSettings {
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                database: "postgres".to_string(),
            },
            password: String::new(),
            file_path: path,
            recreate_database: true,
        };
        let result = restorer.restore(&request, &Cancellation::new(), &|_progress| {});
        assert!(matches!(result, Err(RestoreError::InvalidRequest(_))));
    }
}
