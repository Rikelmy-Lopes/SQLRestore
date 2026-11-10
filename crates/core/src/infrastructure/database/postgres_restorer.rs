//! PostgreSQL restore adapter.

use std::fs::{self, File};
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use postgres::{CancelToken, Client, NoTls};

use crate::domain::cancellation::Cancellation;
use crate::domain::config::ConnectionSettings;
use crate::domain::error::RestoreError;
use crate::domain::restore::{
    RestoreProgress, RestoreReport, RestoreRequest, is_protected_database,
};
use crate::domain::sql_script::{CopyDataOutcome, ScriptStatement, SqlScriptReader};
use crate::ports::database_restorer::DatabaseRestorer;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAINTENANCE_DATABASE: &str = "postgres";
const PROGRESS_INTERVAL: u64 = 1024 * 1024;

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
        Self::connect_to(&request.connection, &request.password, database)
    }

    fn connect_to(
        connection: &ConnectionSettings,
        password: &str,
        database: &str,
    ) -> Result<Client, RestoreError> {
        let mut config = postgres::Config::new();
        config
            .host(&connection.host)
            .port(connection.port)
            .user(&connection.user)
            .dbname(database)
            .application_name("SQLRestore")
            .connect_timeout(CONNECT_TIMEOUT);
        if !password.is_empty() {
            config.password(password);
        }
        config
            .connect(NoTls)
            .map_err(|error| RestoreError::Connection(error.to_string()))
    }

    fn set_cancel_token(&self, token: CancelToken) -> Result<(), RestoreError> {
        let mut current = self
            .cancel_token
            .lock()
            .map_err(|_| RestoreError::Cancel("cancel state is poisoned".to_string()))?;
        *current = Some(token);
        Ok(())
    }

    fn clear_cancel_token(&self) {
        if let Ok(mut token) = self.cancel_token.lock() {
            *token = None;
        }
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
}

impl DatabaseRestorer for PostgresRestorer {
    fn restore(
        &self,
        request: &RestoreRequest,
        cancellation: &Cancellation,
        progress: &dyn Fn(RestoreProgress),
    ) -> Result<RestoreReport, RestoreError> {
        request.validate()?;
        let total_bytes = fs::metadata(&request.file_path)?.len();
        let file = File::open(&request.file_path)?;
        let mut script = SqlScriptReader::new(file);

        if cancellation.is_cancelled() {
            return Err(RestoreError::Cancelled);
        }

        if request.recreate_database {
            Self::recreate_database(request)?;
        }

        let mut client = Self::connect(request, &request.connection.database)?;
        self.set_cancel_token(client.cancel_token())?;

        let outcome = run_chunks(
            &mut client,
            &mut script,
            total_bytes,
            cancellation,
            progress,
        );

        self.clear_cancel_token();

        outcome
    }

    fn execute_script(
        &self,
        connection: &ConnectionSettings,
        password: &str,
        script: &str,
        cancellation: &Cancellation,
    ) -> Result<(), RestoreError> {
        if script.trim().is_empty() {
            return Ok(());
        }
        if cancellation.is_cancelled() {
            return Err(RestoreError::Cancelled);
        }

        let mut client = Self::connect_to(connection, password, &connection.database)?;
        self.set_cancel_token(client.cancel_token())?;

        let total_bytes = u64::try_from(script.len()).unwrap_or(u64::MAX);
        let mut reader = SqlScriptReader::new(script.as_bytes());
        let outcome = run_chunks(
            &mut client,
            &mut reader,
            total_bytes,
            cancellation,
            &|_progress| {},
        );

        self.clear_cancel_token();

        outcome.map(|_report| ())
    }

    fn test_connection(
        &self,
        connection: &ConnectionSettings,
        password: &str,
        recreate_database: bool,
    ) -> Result<(), RestoreError> {
        if connection.host.trim().is_empty() {
            return Err(RestoreError::InvalidRequest("host is empty".to_string()));
        }
        if connection.port == 0 {
            return Err(RestoreError::InvalidRequest("port is empty".to_string()));
        }
        if connection.user.trim().is_empty() {
            return Err(RestoreError::InvalidRequest("user is empty".to_string()));
        }
        if connection.database.trim().is_empty() {
            return Err(RestoreError::InvalidRequest(
                "database is empty".to_string(),
            ));
        }

        let database = if recreate_database {
            MAINTENANCE_DATABASE
        } else {
            &connection.database
        };
        Self::connect_to(connection, password, database)?;
        Ok(())
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

fn run_chunks<R: Read>(
    client: &mut Client,
    script: &mut SqlScriptReader<R>,
    total_bytes: u64,
    cancellation: &Cancellation,
    progress: &dyn Fn(RestoreProgress),
) -> Result<RestoreReport, RestoreError> {
    let mut statements = 0usize;
    let mut last_progress = 0u64;

    let mut report_progress = |offset: u64| {
        if offset.saturating_sub(last_progress) >= PROGRESS_INTERVAL || offset >= total_bytes {
            last_progress = offset;
            progress(RestoreProgress {
                processed_bytes: offset.min(total_bytes),
                total_bytes,
            });
        }
    };

    while let Some(statement) = script.next_statement()? {
        if cancellation.is_cancelled() {
            return Err(RestoreError::Cancelled);
        }

        if statement.copy {
            let query = statement.sql.trim_end().trim_end_matches(';').trim_end();
            let mut writer = client
                .copy_in(query)
                .map_err(|error| database_error(&statement, error))?;
            let outcome = script
                .read_copy_data(&mut writer, &mut |offset| {
                    report_progress(offset);
                    !cancellation.is_cancelled()
                })
                .map_err(|error| database_io_error(&statement, error))?;
            if outcome == CopyDataOutcome::Cancelled {
                return Err(RestoreError::Cancelled);
            }
            writer
                .finish()
                .map_err(|error| database_error(&statement, error))?;
        } else {
            client
                .batch_execute(&statement.sql)
                .map_err(|error| database_error(&statement, error))?;
        }

        statements += 1;
        report_progress(script.position());
    }

    report_progress(total_bytes);

    Ok(RestoreReport {
        statements,
        bytes: total_bytes,
        elapsed: Duration::ZERO,
    })
}

fn database_error(statement: &ScriptStatement, error: postgres::Error) -> RestoreError {
    RestoreError::Database {
        line: statement_line(statement, &error),
        message: describe_database_error(&error),
    }
}

fn database_io_error(statement: &ScriptStatement, error: std::io::Error) -> RestoreError {
    RestoreError::Database {
        line: statement.start_line,
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
fn statement_line(statement: &ScriptStatement, error: &postgres::Error) -> usize {
    let position = error
        .as_db_error()
        .and_then(|database_error| database_error.position())
        .and_then(|position| match position {
            postgres::error::ErrorPosition::Original(position) => Some(*position as usize),
            postgres::error::ErrorPosition::Internal { .. } => None,
        });

    statement.start_line + position.map_or(0, |position| newlines_before(&statement.sql, position))
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
    fn blank_script_does_not_connect() {
        let restorer = PostgresRestorer::new();
        let connection = crate::domain::config::ConnectionSettings::default();
        let result = restorer.execute_script(&connection, "", "  \n", &Cancellation::new());
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_request_is_rejected_before_connecting() {
        let restorer = PostgresRestorer::new();
        let request = RestoreRequest {
            connection: crate::domain::config::ConnectionSettings::default(),
            password: String::new(),
            file_path: "missing.sql".into(),
            recreate_database: false,
            after_restore_script: None,
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
            after_restore_script: None,
        };
        let result = restorer.restore(&request, &Cancellation::new(), &|_progress| {});
        assert!(matches!(result, Err(RestoreError::InvalidRequest(_))));
    }
}
