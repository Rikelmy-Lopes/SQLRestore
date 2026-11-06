//! Restore request, progress and result.

use std::fmt;
use std::path::PathBuf;

use super::config::ConnectionSettings;
use super::error::RestoreError;

/// Input needed to restore a backup.
#[derive(Clone, PartialEq, Eq)]
pub struct RestoreRequest {
    pub connection: ConnectionSettings,
    pub password: String,
    pub file_path: PathBuf,
    /// Drops and recreates the target database before restoring.
    pub recreate_database: bool,
}

impl fmt::Debug for RestoreRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RestoreRequest")
            .field("connection", &self.connection)
            .field("password", &"***")
            .field("file_path", &self.file_path)
            .field("recreate_database", &self.recreate_database)
            .finish()
    }
}

impl RestoreRequest {
    /// Checks that the connection values and the backup file are usable.
    pub fn validate(&self) -> Result<(), RestoreError> {
        if self.connection.host.trim().is_empty() {
            return Err(RestoreError::InvalidRequest("host is empty".to_string()));
        }
        if self.connection.port == 0 {
            return Err(RestoreError::InvalidRequest("port is empty".to_string()));
        }
        if self.connection.user.trim().is_empty() {
            return Err(RestoreError::InvalidRequest("user is empty".to_string()));
        }
        if self.connection.database.trim().is_empty() {
            return Err(RestoreError::InvalidRequest(
                "database is empty".to_string(),
            ));
        }
        if !self.file_path.is_file() {
            return Err(RestoreError::InvalidRequest(
                "backup file does not exist".to_string(),
            ));
        }
        if self.recreate_database && is_protected_database(&self.connection.database) {
            return Err(RestoreError::InvalidRequest(format!(
                "refusing to delete the maintenance database \"{}\"",
                self.connection.database
            )));
        }
        Ok(())
    }
}

/// Database names that must never be deleted automatically.
pub fn is_protected_database(name: &str) -> bool {
    matches!(name.trim(), "postgres" | "template0" | "template1")
}

/// Progress of a running restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
}

impl RestoreProgress {
    /// Completion ratio between 0 and 1.
    pub fn fraction(self) -> f32 {
        if self.total_bytes == 0 {
            return 1.0;
        }
        (self.processed_bytes as f32 / self.total_bytes as f32).clamp(0.0, 1.0)
    }
}

/// Summary of a finished restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreReport {
    pub statements: usize,
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> RestoreRequest {
        let (_file, path) = tempfile::NamedTempFile::new()
            .expect("temporary file")
            .keep()
            .expect("keep temporary file");
        RestoreRequest {
            connection: ConnectionSettings {
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                database: "app".to_string(),
            },
            password: "secret".to_string(),
            file_path: path,
            recreate_database: false,
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(valid_request().validate().is_ok());
    }

    #[test]
    fn empty_database_is_rejected() {
        let mut request = valid_request();
        request.connection.database.clear();
        assert!(request.validate().is_err());
    }

    #[test]
    fn missing_file_is_rejected() {
        let mut request = valid_request();
        request.file_path = PathBuf::from("definitely-missing.sql");
        assert!(request.validate().is_err());
    }

    #[test]
    fn debug_does_not_show_password() {
        let request = valid_request();
        let text = format!("{request:?}");
        assert!(!text.contains("secret"));
        assert!(text.contains("***"));
    }

    #[test]
    fn recreate_rejects_maintenance_databases() {
        let mut request = valid_request();
        request.recreate_database = true;
        for database in ["postgres", "template0", "template1"] {
            request.connection.database = database.to_string();
            assert!(
                request.validate().is_err(),
                "database {database} must be protected"
            );
        }
    }

    #[test]
    fn recreate_accepts_a_normal_database() {
        let mut request = valid_request();
        request.recreate_database = true;
        request.connection.database = "app".to_string();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn progress_fraction_is_clamped() {
        assert_eq!(
            RestoreProgress {
                processed_bytes: 50,
                total_bytes: 100
            }
            .fraction(),
            0.5
        );
        assert_eq!(
            RestoreProgress {
                processed_bytes: 100,
                total_bytes: 0
            }
            .fraction(),
            1.0
        );
    }
}
