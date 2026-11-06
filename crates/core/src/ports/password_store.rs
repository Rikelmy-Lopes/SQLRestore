//! Password storage port.

use crate::domain::config::ConnectionSettings;
use crate::domain::error::PasswordError;

/// Saves and reads connection passwords in the system keychain.
pub trait PasswordStore: Send + Sync {
    /// Saves the password. It replaces an existing password.
    fn save(&self, connection: &ConnectionSettings, password: &str) -> Result<(), PasswordError>;

    /// Returns the saved password, or `None` when no entry exists.
    fn load(&self, connection: &ConnectionSettings) -> Result<Option<String>, PasswordError>;

    /// Removes the saved password. A missing entry is not an error.
    fn delete(&self, connection: &ConnectionSettings) -> Result<(), PasswordError>;
}
