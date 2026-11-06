//! Save configuration command.

use std::sync::Arc;

use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;
use crate::ports::config_repository::ConfigRepository;

/// User action: save the configuration.
pub struct SaveConfigCommand {
    pub config: AppConfig,
}

/// Handles [`SaveConfigCommand`].
pub struct SaveConfigCommandHandler {
    repository: Arc<dyn ConfigRepository>,
}

impl SaveConfigCommandHandler {
    pub fn new(repository: Arc<dyn ConfigRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, command: SaveConfigCommand) -> Result<(), ConfigError> {
        self.repository.save(&command.config)
    }
}
