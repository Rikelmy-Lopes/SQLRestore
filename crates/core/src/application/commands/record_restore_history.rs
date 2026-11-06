//! Record a finished restore in the history command.

use std::sync::Arc;

use crate::domain::error::HistoryError;
use crate::domain::history::{RestoreHistoryEntry, push_entry};
use crate::ports::history_repository::HistoryRepository;

/// User action: add an entry to the restore history.
pub struct RecordRestoreHistoryCommand {
    pub entry: RestoreHistoryEntry,
}

/// Handles [`RecordRestoreHistoryCommand`].
pub struct RecordRestoreHistoryCommandHandler {
    repository: Arc<dyn HistoryRepository>,
}

impl RecordRestoreHistoryCommandHandler {
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, command: RecordRestoreHistoryCommand) -> Result<(), HistoryError> {
        let mut entries = self.repository.load()?;
        push_entry(&mut entries, command.entry);
        self.repository.save(&entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::history::MAX_HISTORY_ENTRIES;
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
    fn records_a_new_entry() {
        let repository = Arc::new(MemoryHistoryRepository::default());
        let handler = RecordRestoreHistoryCommandHandler::new(repository.clone());

        handler
            .execute(RecordRestoreHistoryCommand {
                entry: entry("shop.sql"),
            })
            .expect("record");

        assert_eq!(repository.load().expect("load"), vec![entry("shop.sql")]);
    }

    #[test]
    fn removes_the_oldest_entries_above_the_limit() {
        let repository = Arc::new(MemoryHistoryRepository::default());
        let existing: Vec<RestoreHistoryEntry> = (0..MAX_HISTORY_ENTRIES)
            .map(|index| entry(&format!("backup-{index}.sql")))
            .collect();
        repository.save(&existing).expect("seed");
        let handler = RecordRestoreHistoryCommandHandler::new(repository.clone());

        handler
            .execute(RecordRestoreHistoryCommand {
                entry: entry("newest.sql"),
            })
            .expect("record");

        let entries = repository.load().expect("load");
        assert_eq!(entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(entries[0].file_name, "backup-1.sql");
        assert_eq!(entries[MAX_HISTORY_ENTRIES - 1].file_name, "newest.sql");
    }
}
