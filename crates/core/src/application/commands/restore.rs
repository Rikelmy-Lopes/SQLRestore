//! Restore command.

use std::sync::Arc;

use crate::domain::cancellation::Cancellation;
use crate::domain::error::RestoreError;
use crate::domain::restore::{RestoreProgress, RestoreReport, RestoreRequest};
use crate::ports::database_restorer::DatabaseRestorer;
use crate::ports::logger::{LogLevel, Logger};

/// User action: restore a backup file.
pub struct RestoreCommand {
    pub request: RestoreRequest,
}

/// Handles [`RestoreCommand`]. It runs outside the interface thread.
pub struct RestoreCommandHandler {
    restorer: Arc<dyn DatabaseRestorer>,
    logger: Arc<dyn Logger>,
    cancellation: Cancellation,
}

impl RestoreCommandHandler {
    pub fn new(
        restorer: Arc<dyn DatabaseRestorer>,
        logger: Arc<dyn Logger>,
        cancellation: Cancellation,
    ) -> Self {
        Self {
            restorer,
            logger,
            cancellation,
        }
    }

    pub fn execute(
        &self,
        command: RestoreCommand,
        progress: &dyn Fn(RestoreProgress),
    ) -> Result<RestoreReport, RestoreError> {
        let request = command.request;
        log_start(self.logger.as_ref(), &request);

        if self.cancellation.is_cancelled() {
            self.logger
                .log(LogLevel::Info, "Restore cancelled before start");
            return Err(RestoreError::Cancelled);
        }

        if request.recreate_database {
            self.logger.log(
                LogLevel::Warning,
                &format!(
                    "Recreating database: database={} (all data will be deleted)",
                    request.connection.database
                ),
            );
        }

        let result = self
            .restorer
            .restore(&request, &self.cancellation, progress);

        match &result {
            Ok(report) => self.logger.log(
                LogLevel::Info,
                &format!(
                    "Restore finished: file={} statements={} bytes={}",
                    request.file_path.display(),
                    report.statements,
                    report.bytes
                ),
            ),
            Err(RestoreError::Cancelled) => self.logger.log(
                LogLevel::Info,
                &format!("Restore cancelled: file={}", request.file_path.display()),
            ),
            Err(error) => self.logger.log(
                LogLevel::Error,
                &format!(
                    "Restore failed: file={} error={error}",
                    request.file_path.display()
                ),
            ),
        }

        result
    }
}

fn log_start(logger: &dyn Logger, request: &RestoreRequest) {
    logger.log(
        LogLevel::Info,
        &format!(
            "Restore started: file={} host={} port={} user={} database={} recreate_database={}",
            request.file_path.display(),
            request.connection.host,
            request.connection.port,
            request.connection.user,
            request.connection.database,
            request.recreate_database
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ConnectionSettings;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

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
        result: Mutex<Option<Result<RestoreReport, RestoreError>>>,
        progress: AtomicBool,
    }

    impl FakeRestorer {
        fn success() -> Self {
            Self {
                result: Mutex::new(Some(Ok(RestoreReport {
                    statements: 2,
                    bytes: 10,
                }))),
                progress: AtomicBool::new(false),
            }
        }

        fn failure() -> Self {
            Self {
                result: Mutex::new(Some(Err(RestoreError::Database {
                    line: 4,
                    message: "syntax error".to_string(),
                }))),
                progress: AtomicBool::new(false),
            }
        }

        fn cancelled() -> Self {
            Self {
                result: Mutex::new(Some(Err(RestoreError::Cancelled))),
                progress: AtomicBool::new(false),
            }
        }
    }

    impl DatabaseRestorer for FakeRestorer {
        fn restore(
            &self,
            _request: &RestoreRequest,
            _cancellation: &Cancellation,
            progress: &dyn Fn(RestoreProgress),
        ) -> Result<RestoreReport, RestoreError> {
            self.progress.store(true, Ordering::SeqCst);
            progress(RestoreProgress {
                processed_bytes: 10,
                total_bytes: 10,
            });
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

    fn request() -> RestoreRequest {
        RestoreRequest {
            connection: ConnectionSettings {
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                database: "app".to_string(),
            },
            password: "top-secret".to_string(),
            file_path: PathBuf::from("backup.sql"),
            recreate_database: false,
        }
    }

    fn handler(restorer: FakeRestorer) -> (RestoreCommandHandler, Arc<MemoryLogger>) {
        let logger = Arc::new(MemoryLogger::default());
        let handler =
            RestoreCommandHandler::new(Arc::new(restorer), logger.clone(), Cancellation::new());
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
        let result = handler.execute(RestoreCommand { request: request() }, &|_| {});
        assert!(result.is_ok());
        let messages = messages(&logger);
        assert!(messages[0].contains("Restore started"));
        assert!(messages.iter().any(|m| m.contains("Restore finished")));
        assert!(!messages.iter().any(|m| m.contains("top-secret")));
    }

    #[test]
    fn failure_logs_error() {
        let (handler, logger) = handler(FakeRestorer::failure());
        let result = handler.execute(RestoreCommand { request: request() }, &|_| {});
        assert!(result.is_err());
        assert!(
            messages(&logger)
                .iter()
                .any(|message| message.contains("Restore failed"))
        );
    }

    #[test]
    fn cancellation_logs_cancel() {
        let (handler, logger) = handler(FakeRestorer::cancelled());
        let result = handler.execute(RestoreCommand { request: request() }, &|_| {});
        assert!(matches!(result, Err(RestoreError::Cancelled)));
        assert!(
            messages(&logger)
                .iter()
                .any(|message| message.contains("Restore cancelled"))
        );
    }

    #[test]
    fn progress_reaches_the_callback() {
        let (handler, _logger) = handler(FakeRestorer::success());
        let seen = std::sync::Arc::new(AtomicBool::new(false));
        let seen_clone = seen.clone();
        let _ = handler.execute(RestoreCommand { request: request() }, &move |progress| {
            assert_eq!(progress.processed_bytes, 10);
            seen_clone.store(true, Ordering::SeqCst);
        });
        assert!(seen.load(Ordering::SeqCst));
    }
}
