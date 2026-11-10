//! Database restore port.

use crate::domain::cancellation::Cancellation;
use crate::domain::config::ConnectionSettings;
use crate::domain::error::RestoreError;
use crate::domain::restore::{RestoreProgress, RestoreReport, RestoreRequest};

/// Restores a backup through a database connection.
pub trait DatabaseRestorer: Send + Sync {
    /// Restores the file. Reports progress after each statement.
    fn restore(
        &self,
        request: &RestoreRequest,
        cancellation: &Cancellation,
        progress: &dyn Fn(RestoreProgress),
    ) -> Result<RestoreReport, RestoreError>;

    /// Runs a user-provided SQL script, such as the after-restore script.
    fn execute_script(
        &self,
        connection: &ConnectionSettings,
        password: &str,
        script: &str,
        cancellation: &Cancellation,
    ) -> Result<(), RestoreError>;

    /// Checks that the connection values can open a database session.
    ///
    /// With `recreate_database`, the target database may not exist yet, so the
    /// maintenance database is used instead.
    fn test_connection(
        &self,
        connection: &ConnectionSettings,
        password: &str,
        recreate_database: bool,
    ) -> Result<(), RestoreError>;

    /// Asks the database to stop the current query.
    fn cancel(&self) -> Result<(), RestoreError>;
}
