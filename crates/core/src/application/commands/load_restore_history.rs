//! Load the restore history command.

use std::sync::Arc;

use crate::domain::error::HistoryError;
use crate::domain::history::RestoreHistoryEntry;
use crate::ports::history_repository::HistoryRepository;

/// User action: load the restore history.
pub struct LoadRestoreHistoryCommand;

/// Handles [`LoadRestoreHistoryCommand`].
pub struct LoadRestoreHistoryCommandHandler {
    repository: Arc<dyn HistoryRepository>,
}

impl LoadRestoreHistoryCommandHandler {
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(
        &self,
        _command: LoadRestoreHistoryCommand,
    ) -> Result<Vec<RestoreHistoryEntry>, HistoryError> {
        self.repository.load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn returns_the_saved_entries() {
        let repository = Arc::new(MemoryHistoryRepository::default());
        let entry = RestoreHistoryEntry {
            file_name: "shop.sql".to_string(),
            database: "shop".to_string(),
            connection: "admin@db.local:5432".to_string(),
            date: "2026-11-06".to_string(),
            time: "10:00:00".to_string(),
            duration_ms: 1500,
        };
        repository.save(std::slice::from_ref(&entry)).expect("save");
        let handler = LoadRestoreHistoryCommandHandler::new(repository);

        assert_eq!(
            handler.execute(LoadRestoreHistoryCommand).expect("load"),
            vec![entry]
        );
    }
}
