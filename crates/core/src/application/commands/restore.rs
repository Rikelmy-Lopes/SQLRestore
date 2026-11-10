//! Restore command.

use std::sync::Arc;
use std::time::Instant;

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

        let started = Instant::now();
        let result = match self
            .restorer
            .restore(&request, &self.cancellation, progress)
        {
            Ok(mut report) => match self.run_after_restore_script(&request) {
                Ok(()) => {
                    report.elapsed = started.elapsed();
                    Ok(report)
                }
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };

        match &result {
            Ok(report) => self.logger.log(
                LogLevel::Info,
                &format!(
                    "Restore finished: file={} statements={} bytes={} elapsed={:?}",
                    request.file_path.display(),
                    report.statements,
                    report.bytes,
                    report.elapsed
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

    /// Runs the optional script after the backup restore has succeeded.
    fn run_after_restore_script(&self, request: &RestoreRequest) -> Result<(), RestoreError> {
        let Some(script) = request.after_restore_script.as_deref() else {
            return Ok(());
        };
        if script.trim().is_empty() {
            return Ok(());
        }

        self.logger
            .log(LogLevel::Info, "After-restore script started");

        match self.restorer.execute_script(
            &request.connection,
            &request.password,
            script,
            &self.cancellation,
        ) {
            Ok(()) => {
                self.logger
                    .log(LogLevel::Info, "After-restore script finished");
                Ok(())
            }
            Err(RestoreError::Cancelled) => {
                self.logger
                    .log(LogLevel::Info, "After-restore script cancelled");
                Err(RestoreError::Cancelled)
            }
            Err(error) => {
                self.logger.log(
                    LogLevel::Error,
                    &format!("After-restore script failed: {error}"),
                );
                Err(RestoreError::AfterRestoreScript(error.to_string()))
            }
        }
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
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
        script_result: Mutex<Option<Result<(), RestoreError>>>,
        progress: AtomicBool,
        script_calls: AtomicUsize,
    }

    impl FakeRestorer {
        fn success() -> Self {
            Self {
                result: Mutex::new(Some(Ok(RestoreReport {
                    statements: 2,
                    bytes: 10,
                    elapsed: std::time::Duration::ZERO,
                }))),
                script_result: Mutex::new(Some(Ok(()))),
                progress: AtomicBool::new(false),
                script_calls: AtomicUsize::new(0),
            }
        }

        fn failure() -> Self {
            Self {
                result: Mutex::new(Some(Err(RestoreError::Database {
                    line: 4,
                    message: "syntax error".to_string(),
                }))),
                script_result: Mutex::new(Some(Ok(()))),
                progress: AtomicBool::new(false),
                script_calls: AtomicUsize::new(0),
            }
        }

        fn script_failure() -> Self {
            let restorer = Self::success();
            *restorer.script_result.lock().expect("script result lock") =
                Some(Err(RestoreError::Database {
                    line: 1,
                    message: "script syntax error".to_string(),
                }));
            restorer
        }

        fn cancelled() -> Self {
            Self {
                result: Mutex::new(Some(Err(RestoreError::Cancelled))),
                script_result: Mutex::new(Some(Ok(()))),
                progress: AtomicBool::new(false),
                script_calls: AtomicUsize::new(0),
            }
        }

        fn script_calls(&self) -> usize {
            self.script_calls.load(Ordering::SeqCst)
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

        fn execute_script(
            &self,
            _connection: &ConnectionSettings,
            _password: &str,
            _script: &str,
            _cancellation: &Cancellation,
        ) -> Result<(), RestoreError> {
            self.script_calls.fetch_add(1, Ordering::SeqCst);
            self.script_result
                .lock()
                .expect("script result lock")
                .take()
                .expect("script result present")
        }

        fn test_connection(
            &self,
            _connection: &ConnectionSettings,
            _password: &str,
            _recreate_database: bool,
        ) -> Result<(), RestoreError> {
            Ok(())
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
            after_restore_script: None,
        }
    }

    fn handler(restorer: FakeRestorer) -> (RestoreCommandHandler, Arc<MemoryLogger>) {
        handler_with(Arc::new(restorer))
    }

    fn handler_with(restorer: Arc<FakeRestorer>) -> (RestoreCommandHandler, Arc<MemoryLogger>) {
        let logger = Arc::new(MemoryLogger::default());
        let handler = RestoreCommandHandler::new(restorer, logger.clone(), Cancellation::new());
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
    fn success_reports_elapsed_time() {
        let (handler, _logger) = handler(FakeRestorer::success());
        let report = handler
            .execute(RestoreCommand { request: request() }, &|_| {})
            .expect("restore succeeds");
        assert!(report.elapsed > std::time::Duration::ZERO);
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

    fn request_with_script(script: &str) -> RestoreRequest {
        let mut request = request();
        request.after_restore_script = Some(script.to_string());
        request
    }

    #[test]
    fn after_restore_script_runs_after_a_successful_restore() {
        let restorer = Arc::new(FakeRestorer::success());
        let (handler, logger) = handler_with(restorer.clone());
        let result = handler.execute(
            RestoreCommand {
                request: request_with_script("SELECT 1;"),
            },
            &|_| {},
        );
        assert!(result.is_ok());
        assert_eq!(restorer.script_calls(), 1);
        assert!(
            messages(&logger)
                .iter()
                .any(|message| message.contains("After-restore script finished"))
        );
    }

    #[test]
    fn after_restore_script_does_not_run_when_the_restore_fails() {
        let restorer = Arc::new(FakeRestorer::failure());
        let (handler, _logger) = handler_with(restorer.clone());
        let result = handler.execute(
            RestoreCommand {
                request: request_with_script("SELECT 1;"),
            },
            &|_| {},
        );
        assert!(result.is_err());
        assert_eq!(restorer.script_calls(), 0);
    }

    #[test]
    fn after_restore_script_does_not_run_when_the_restore_is_cancelled() {
        let restorer = Arc::new(FakeRestorer::cancelled());
        let (handler, _logger) = handler_with(restorer.clone());
        let result = handler.execute(
            RestoreCommand {
                request: request_with_script("SELECT 1;"),
            },
            &|_| {},
        );
        assert!(matches!(result, Err(RestoreError::Cancelled)));
        assert_eq!(restorer.script_calls(), 0);
    }

    #[test]
    fn after_restore_script_failure_fails_the_command() {
        let (handler, logger) = handler(FakeRestorer::script_failure());
        let result = handler.execute(
            RestoreCommand {
                request: request_with_script("SELECT 1;"),
            },
            &|_| {},
        );
        assert!(matches!(
            result,
            Err(RestoreError::AfterRestoreScript(message)) if message.contains("script syntax error")
        ));
        assert!(
            messages(&logger)
                .iter()
                .any(|message| message.contains("After-restore script failed"))
        );
    }

    #[test]
    fn blank_after_restore_script_is_ignored() {
        let restorer = Arc::new(FakeRestorer::success());
        let (handler, _logger) = handler_with(restorer.clone());
        let result = handler.execute(
            RestoreCommand {
                request: request_with_script("   \n\t"),
            },
            &|_| {},
        );
        assert!(result.is_ok());
        assert_eq!(restorer.script_calls(), 0);
    }
}
