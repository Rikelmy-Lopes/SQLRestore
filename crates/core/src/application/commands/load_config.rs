//! Load configuration command.

use std::sync::Arc;

use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;
use crate::ports::config_repository::ConfigRepository;

/// User action: load the configuration.
pub struct LoadConfigCommand;

/// Handles [`LoadConfigCommand`].
pub struct LoadConfigCommandHandler {
    repository: Arc<dyn ConfigRepository>,
}

impl LoadConfigCommandHandler {
    pub fn new(repository: Arc<dyn ConfigRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, _command: LoadConfigCommand) -> Result<AppConfig, ConfigError> {
        self.repository.load()
    }
}
