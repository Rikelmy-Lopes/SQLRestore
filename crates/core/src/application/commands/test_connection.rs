//! Test connection command.

use std::sync::Arc;

use crate::domain::config::ConnectionSettings;
use crate::domain::error::RestoreError;
use crate::ports::database_restorer::DatabaseRestorer;
use crate::ports::logger::{LogLevel, Logger};

/// User action: check the database connection before restoring.
pub struct TestConnectionCommand {
    pub connection: ConnectionSettings,
    pub password: String,
    /// The target database may not exist when it will be recreated.
    pub recreate_database: bool,
}

/// Handles [`TestConnectionCommand`]. It runs outside the interface thread.
pub struct TestConnectionCommandHandler {
    restorer: Arc<dyn DatabaseRestorer>,
    logger: Arc<dyn Logger>,
}

impl TestConnectionCommandHandler {
    pub fn new(restorer: Arc<dyn DatabaseRestorer>, logger: Arc<dyn Logger>) -> Self {
        Self { restorer, logger }
    }

    pub fn execute(&self, command: TestConnectionCommand) -> Result<(), RestoreError> {
        let connection = command.connection;
        self.logger.log(
            LogLevel::Info,
            &format!(
                "Connection test started: host={} port={} user={} database={}",
                connection.host, connection.port, connection.user, connection.database
            ),
        );

        let result = self.restorer.test_connection(
            &connection,
            &command.password,
            command.recreate_database,
        );

        match &result {
            Ok(()) => self.logger.log(LogLevel::Info, "Connection test succeeded"),
            Err(error) => self
                .logger
                .log(LogLevel::Error, &format!("Connection test failed: {error}")),
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryLogger {
        entries: Mutex<Vec<(LogLevel, String)>>,
    }

    impl Logger for MemoryLogger {
        fn log(&self, level: LogLevel, message: &str) {
            self.entries
                .lock()
                .expect("logger lock")
                .push((level, message.to_string()));
        }
    }

    struct FakeRestorer {
        result: Mutex<Option<Result<(), RestoreError>>>,
    }

    impl FakeRestorer {
        fn success() -> Self {
            Self {
                result: Mutex::new(Some(Ok(()))),
            }
        }

        fn failure() -> Self {
            Self {
                result: Mutex::new(Some(Err(RestoreError::Connection(
                    "password authentication failed".to_string(),
                )))),
            }
        }
    }

    impl DatabaseRestorer for FakeRestorer {
        fn restore(
            &self,
            _request: &crate::domain::restore::RestoreRequest,
            _cancellation: &crate::domain::cancellation::Cancellation,
            _progress: &dyn Fn(crate::domain::restore::RestoreProgress),
        ) -> Result<crate::domain::restore::RestoreReport, RestoreError> {
            unreachable!("the restore is not used by the connection test")
        }

        fn test_connection(
            &self,
            _connection: &ConnectionSettings,
            _password: &str,
            _recreate_database: bool,
        ) -> Result<(), RestoreError> {
            self.result
                .lock()
                .expect("result lock")
                .take()
                .expect("result present")
        }

        fn cancel(&self) -> Result<(), RestoreError> {
            Ok(())
        }
    }

    fn command() -> TestConnectionCommand {
        TestConnectionCommand {
            connection: ConnectionSettings {
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                database: "app".to_string(),
            },
            password: "top-secret".to_string(),
            recreate_database: false,
        }
    }

    fn handler(restorer: FakeRestorer) -> (TestConnectionCommandHandler, Arc<MemoryLogger>) {
        let logger = Arc::new(MemoryLogger::default());
        let handler = TestConnectionCommandHandler::new(Arc::new(restorer), logger.clone());
        (handler, logger)
    }

    fn messages(logger: &MemoryLogger) -> Vec<String> {
        logger
            .entries
            .lock()
            .expect("logger lock")
            .iter()
            .map(|(_, message)| message.clone())
            .collect()
    }

    #[test]
    fn success_logs_start_and_finish() {
        let (handler, logger) = handler(FakeRestorer::success());
        assert!(handler.execute(command()).is_ok());
        let messages = messages(&logger);
        assert!(messages[0].contains("Connection test started"));
        assert!(
            messages
                .iter()
                .any(|message| message.contains("Connection test succeeded"))
        );
        assert!(
            !messages
                .iter()
                .any(|message| message.contains("top-secret"))
        );
    }

    #[test]
    fn failure_is_returned_and_logged() {
        let (handler, logger) = handler(FakeRestorer::failure());
        let result = handler.execute(command());
        assert!(matches!(result, Err(RestoreError::Connection(_))));
        assert!(
            messages(&logger)
                .iter()
                .any(|message| message.contains("Connection test failed"))
        );
    }
}
