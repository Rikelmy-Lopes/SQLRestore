//! Database restore port.

use crate::domain::cancellation::Cancellation;
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

    /// Asks the database to stop the current query.
    fn cancel(&self) -> Result<(), RestoreError>;
}
