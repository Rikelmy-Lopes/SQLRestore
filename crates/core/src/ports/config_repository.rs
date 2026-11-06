//! Configuration storage port.

use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;

/// Reads and writes the application configuration.
pub trait ConfigRepository: Send + Sync {
    /// Loads the configuration. Missing or invalid values use defaults.
    fn load(&self) -> Result<AppConfig, ConfigError>;

    /// Saves the complete configuration.
    fn save(&self, config: &AppConfig) -> Result<(), ConfigError>;
}
