//! Errors returned by the core.

use thiserror::Error;

/// Error raised while reading or writing the configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not access the configuration file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse the configuration file: {0}")]
    Parse(String),
}

/// Error raised while reading or writing the restore history.
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("could not access the history file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse the history file: {0}")]
    Parse(String),
}

/// Error raised while using the system keychain.
#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("the system keychain is not available: {0}")]
    Keychain(String),
}

/// Error raised while restoring a backup.
#[derive(Debug, Error)]
pub enum RestoreError {
    #[error("the restore request is not valid: {0}")]
    InvalidRequest(String),
    #[error("could not read the SQL file: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("could not connect to the database: {0}")]
    Connection(String),
    #[error("database error at line {line}: {message}")]
    Database { line: usize, message: String },
    #[error("could not recreate the database: {0}")]
    DatabaseRecreate(String),
    #[error("the restore was cancelled")]
    Cancelled,
    #[error("the database query could not be cancelled: {0}")]
    Cancel(String),
}

/// Error raised while loading translations.
#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("could not read the translation directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse translation file {file}: {message}")]
    Parse { file: String, message: String },
}
