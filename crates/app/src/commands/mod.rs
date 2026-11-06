//! Interface commands that connect widgets to core handlers.

mod restore_task;
mod select_backup;

pub use restore_task::{RestoreEvent, RestoreOutcome, RestoreTask};
pub use select_backup::SelectBackupCommandHandler;

use std::sync::Arc;

use sqlrestore_core::application::commands::{
    CancelRestoreCommandHandler, LoadConfigCommand, LoadConfigCommandHandler, LoadPasswordCommand,
    LoadPasswordCommandHandler, RestoreCommandHandler, SaveConfigCommand, SaveConfigCommandHandler,
    SavePasswordCommand, SavePasswordCommandHandler,
};
use sqlrestore_core::domain::cancellation::Cancellation;
use sqlrestore_core::domain::config::{AppConfig, ConnectionSettings};
use sqlrestore_core::domain::error::{ConfigError, PasswordError};
use sqlrestore_core::domain::restore::RestoreRequest;
use sqlrestore_core::ports::config_repository::ConfigRepository;
use sqlrestore_core::ports::database_restorer::DatabaseRestorer;
use sqlrestore_core::ports::logger::{LogLevel, Logger};
use sqlrestore_core::ports::password_store::PasswordStore;

use restore_task::spawn_restore;

/// All handlers used by the main screen.
pub struct AppCommands {
    load_config: LoadConfigCommandHandler,
    save_config: SaveConfigCommandHandler,
    load_password: LoadPasswordCommandHandler,
    save_password: SavePasswordCommandHandler,
    restore: Arc<RestoreCommandHandler>,
    cancel: Arc<CancelRestoreCommandHandler>,
    cancellation: Cancellation,
    logger: Arc<dyn Logger>,
}

impl AppCommands {
    pub fn new(
        config_repository: Arc<dyn ConfigRepository>,
        password_store: Arc<dyn PasswordStore>,
        restorer: Arc<dyn DatabaseRestorer>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        let cancellation = Cancellation::new();
        Self {
            load_config: LoadConfigCommandHandler::new(config_repository.clone()),
            save_config: SaveConfigCommandHandler::new(config_repository),
            load_password: LoadPasswordCommandHandler::new(password_store.clone()),
            save_password: SavePasswordCommandHandler::new(password_store),
            restore: Arc::new(RestoreCommandHandler::new(
                restorer.clone(),
                logger.clone(),
                cancellation.clone(),
            )),
            cancel: Arc::new(CancelRestoreCommandHandler::new(
                restorer,
                logger.clone(),
                cancellation.clone(),
            )),
            cancellation,
            logger,
        }
    }

    pub fn load_config(&self) -> Result<AppConfig, ConfigError> {
        self.load_config.execute(LoadConfigCommand)
    }

    pub fn save_config(&self, config: AppConfig) -> Result<(), ConfigError> {
        self.save_config.execute(SaveConfigCommand { config })
    }

    pub fn load_password(
        &self,
        connection: &ConnectionSettings,
    ) -> Result<Option<String>, PasswordError> {
        self.load_password.execute(LoadPasswordCommand {
            connection: connection.clone(),
        })
    }

    pub fn save_password(
        &self,
        connection: &ConnectionSettings,
        password: &str,
    ) -> Result<(), PasswordError> {
        self.save_password.execute(SavePasswordCommand {
            connection: connection.clone(),
            password: password.to_string(),
        })
    }

    pub fn start_restore(&self, request: RestoreRequest) -> RestoreTask {
        self.cancellation.reset();
        spawn_restore(Arc::clone(&self.restore), Arc::clone(&self.cancel), request)
    }

    /// Writes a non-fatal warning to the log.
    pub fn log_warning(&self, message: &str) {
        self.logger.log(LogLevel::Warning, message);
    }
}
