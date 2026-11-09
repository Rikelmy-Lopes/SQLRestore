//! Clear the restore history command.

use std::sync::Arc;

use crate::domain::error::HistoryError;
use crate::ports::history_repository::HistoryRepository;

/// User action: remove every entry from the restore history.
pub struct ClearRestoreHistoryCommand;

/// Handles [`ClearRestoreHistoryCommand`].
pub struct ClearRestoreHistoryCommandHandler {
    repository: Arc<dyn HistoryRepository>,
}

impl ClearRestoreHistoryCommandHandler {
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, _command: ClearRestoreHistoryCommand) -> Result<(), HistoryError> {
        self.repository.save(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::history::RestoreHistoryEntry;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryHistoryRepository {
        entries: Mutex<Vec<RestoreHistoryEntry>>,
    }

    impl HistoryRepository for MemoryHistoryRepository {
        fn load(&self) -> Result<Vec<RestoreHistoryEntry>, HistoryError> {
            Ok(self.entries.lock().expect("history lock").clone())
        }

        fn save(&self, entries: &[RestoreHistoryEntry]) -> Result<(), HistoryError> {
            *self.entries.lock().expect("history lock") = entries.to_vec();
            Ok(())
        }
    }

    fn entry(file_name: &str) -> RestoreHistoryEntry {
        RestoreHistoryEntry {
            file_name: file_name.to_string(),
            database: "shop".to_string(),
            connection: "admin@db.local:5432".to_string(),
            date: "2026-11-06".to_string(),
            time: "10:00:00".to_string(),
            duration_ms: 1500,
        }
    }

    #[test]
    fn removes_every_entry() {
        let repository = Arc::new(MemoryHistoryRepository::default());
        repository
            .save(&[entry("shop.sql"), entry("blog.sql")])
            .expect("seed");
        let handler = ClearRestoreHistoryCommandHandler::new(repository.clone());

        handler
            .execute(ClearRestoreHistoryCommand)
            .expect("clear history");

        assert!(repository.load().expect("load").is_empty());
    }

    #[test]
    fn clearing_an_empty_history_succeeds() {
        let repository = Arc::new(MemoryHistoryRepository::default());
        let handler = ClearRestoreHistoryCommandHandler::new(repository);

        handler
            .execute(ClearRestoreHistoryCommand)
            .expect("clear history");
    }
}
