//! File-backed restore history repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::error::HistoryError;
use crate::domain::history::RestoreHistoryEntry;
use crate::infrastructure::paths;
use crate::ports::history_repository::HistoryRepository;

/// Stores the restore history in a JSON array.
pub struct FileHistoryRepository {
    path: PathBuf,
}

impl FileHistoryRepository {
    /// Uses the given file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Uses the platform AppData location.
    pub fn default_location() -> Self {
        Self::new(paths::history_path())
    }

    /// File used by this repository.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl HistoryRepository for FileHistoryRepository {
    fn load(&self) -> Result<Vec<RestoreHistoryEntry>, HistoryError> {
        if !self.path.is_file() {
            return Ok(Vec::new());
        }

        let raw = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn save(&self, entries: &[RestoreHistoryEntry]) -> Result<(), HistoryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut json = serde_json::to_string_pretty(entries)
            .map_err(|error| HistoryError::Parse(error.to_string()))?;
        json.push('\n');
        fs::write(&self.path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn repository() -> (tempfile::TempDir, FileHistoryRepository) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("history.json");
        (directory, FileHistoryRepository::new(path))
    }

    #[test]
    fn missing_file_is_empty() {
        let (_directory, repository) = repository();
        assert!(repository.load().expect("load").is_empty());
    }

    #[test]
    fn round_trips_entries() {
        let (_directory, repository) = repository();
        let entries = vec![entry("shop.sql"), entry("blog.sql")];
        repository.save(&entries).expect("save");
        assert_eq!(repository.load().expect("load"), entries);
    }

    #[test]
    fn creates_the_parent_directory() {
        let (directory, _repository) = repository();
        let nested =
            FileHistoryRepository::new(directory.path().join("nested").join("history.json"));
        nested.save(&[entry("shop.sql")]).expect("save");
        assert!(nested.path().is_file());
    }

    #[test]
    fn invalid_file_is_empty() {
        let (directory, repository) = repository();
        fs::write(directory.path().join("history.json"), "not json").expect("write");
        assert!(repository.load().expect("load").is_empty());
    }
}
