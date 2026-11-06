//! System keychain password store.

use crate::domain::config::ConnectionSettings;
use crate::domain::error::PasswordError;
use crate::ports::password_store::PasswordStore;

/// Service name used by every keychain entry.
pub const SERVICE_NAME: &str = "SQLRestore";

/// Entry name built from the connection values.
pub fn entry_name(connection: &ConnectionSettings) -> String {
    format!(
        "{}:{}:{}",
        connection.host, connection.port, connection.user
    )
}

/// Stores passwords in the system keychain.
pub struct KeyringPasswordStore;

impl KeyringPasswordStore {
    fn entry(connection: &ConnectionSettings) -> Result<keyring::Entry, PasswordError> {
        keyring::Entry::new(SERVICE_NAME, &entry_name(connection))
            .map_err(|error| PasswordError::Keychain(error.to_string()))
    }
}

impl PasswordStore for KeyringPasswordStore {
    fn save(&self, connection: &ConnectionSettings, password: &str) -> Result<(), PasswordError> {
        Self::entry(connection)?
            .set_password(password)
            .map_err(|error| PasswordError::Keychain(error.to_string()))
    }

    fn load(&self, connection: &ConnectionSettings) -> Result<Option<String>, PasswordError> {
        match Self::entry(connection)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(PasswordError::Keychain(error.to_string())),
        }
    }

    fn delete(&self, connection: &ConnectionSettings) -> Result<(), PasswordError> {
        match Self::entry(connection)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(PasswordError::Keychain(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_name_uses_connection_values() {
        let connection = ConnectionSettings {
            host: "db.local".to_string(),
            port: 6543,
            user: "admin".to_string(),
            database: "shop".to_string(),
        };
        assert_eq!(entry_name(&connection), "db.local:6543:admin");
    }

    #[test]
    fn entry_name_has_no_password() {
        let connection = ConnectionSettings::default();
        assert!(!entry_name(&connection).contains("password"));
    }
}
