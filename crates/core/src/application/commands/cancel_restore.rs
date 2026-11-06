//! Cancel restore command.

use std::sync::Arc;

use crate::domain::cancellation::Cancellation;
use crate::domain::error::RestoreError;
use crate::ports::database_restorer::DatabaseRestorer;
use crate::ports::logger::{LogLevel, Logger};

/// User action: cancel the running restore.
pub struct CancelRestoreCommand;

/// Handles [`CancelRestoreCommand`].
pub struct CancelRestoreCommandHandler {
    restorer: Arc<dyn DatabaseRestorer>,
    logger: Arc<dyn Logger>,
    cancellation: Cancellation,
}

impl CancelRestoreCommandHandler {
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

    pub fn execute(&self, _command: CancelRestoreCommand) -> Result<(), RestoreError> {
        self.cancellation.cancel();
        self.logger
            .log(LogLevel::Info, "Restore cancellation requested");

        if let Err(error) = self.restorer.cancel() {
            self.logger
                .log(LogLevel::Error, &format!("Cancel failed: {error}"));
            return Err(error);
        }
        Ok(())
    }
}
