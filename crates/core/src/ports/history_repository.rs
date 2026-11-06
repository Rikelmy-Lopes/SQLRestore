//! Restore history storage port.

use crate::domain::error::HistoryError;
use crate::domain::history::RestoreHistoryEntry;

/// Reads and writes the restore history.
pub trait HistoryRepository: Send + Sync {
    /// Loads the entries, oldest first. A missing or invalid file returns an
    /// empty list.
    fn load(&self) -> Result<Vec<RestoreHistoryEntry>, HistoryError>;

    /// Saves the complete list, oldest first.
    fn save(&self, entries: &[RestoreHistoryEntry]) -> Result<(), HistoryError>;
}
