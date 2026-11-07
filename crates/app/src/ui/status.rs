//! State of the result area.

use sqlrestore_core::domain::restore::RestoreReport;

/// Current restore state shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idle,
    Running,
    Success(RestoreReport),
    Cancelled,
    Failed(String),
}

/// Current connection test state shown next to its button.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Idle,
    Testing,
    Success,
    Failed(String),
}
