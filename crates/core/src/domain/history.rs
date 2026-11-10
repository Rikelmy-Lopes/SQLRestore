//! Restore history model.

use serde::{Deserialize, Serialize};

use chrono::{DateTime, Local};

use super::restore::{RestoreReport, RestoreRequest};

/// Maximum number of entries kept in the history file.
pub const MAX_HISTORY_ENTRIES: usize = 100;

/// One successful restore, saved in the history file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreHistoryEntry {
    /// Backup file name without its directory.
    pub file_name: String,
    /// Target database.
    pub database: String,
    /// Connection used by the restore.
    pub connection: String,
    /// Local date, formatted as `YYYY-MM-DD`.
    pub date: String,
    /// Local time, formatted as `HH:MM:SS`.
    pub time: String,
    /// Restore duration in milliseconds.
    pub duration_ms: u64,
}

impl RestoreHistoryEntry {
    /// Builds an entry from a restore that just finished.
    pub fn now_from_restore(request: &RestoreRequest, report: &RestoreReport) -> Self {
        Self::from_restore(request, report, Local::now())
    }

    /// Builds an entry from a finished restore.
    pub fn from_restore(
        request: &RestoreRequest,
        report: &RestoreReport,
        completed_at: DateTime<Local>,
    ) -> Self {
        let file_name = request.file_path.file_name().map_or_else(
            || request.file_path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        Self {
            file_name,
            database: request.connection.database.clone(),
            connection: format!(
                "{}@{}:{}",
                request.connection.user, request.connection.host, request.connection.port
            ),
            date: completed_at.format("%Y-%m-%d").to_string(),
            time: completed_at.format("%H:%M:%S").to_string(),
            duration_ms: u64::try_from(report.elapsed.as_millis()).unwrap_or(u64::MAX),
        }
    }
}

/// Adds an entry and removes the oldest entries above the limit.
pub fn push_entry(entries: &mut Vec<RestoreHistoryEntry>, entry: RestoreHistoryEntry) {
    entries.push(entry);
    if entries.len() > MAX_HISTORY_ENTRIES {
        let excess = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ConnectionSettings;
    use std::path::PathBuf;
    use std::time::Duration;

    fn request() -> RestoreRequest {
        RestoreRequest {
            connection: ConnectionSettings {
                host: "db.local".to_string(),
                port: 6543,
                user: "admin".to_string(),
                database: "shop".to_string(),
            },
            password: "secret".to_string(),
            file_path: PathBuf::from("C:/backups/shop.sql"),
            recreate_database: false,
            after_restore_script: None,
        }
    }

    fn report() -> RestoreReport {
        RestoreReport {
            statements: 3,
            bytes: 42,
            elapsed: Duration::from_millis(1500),
        }
    }

    fn entry(index: usize) -> RestoreHistoryEntry {
        RestoreHistoryEntry {
            file_name: format!("backup-{index}.sql"),
            database: "shop".to_string(),
            connection: "admin@db.local:6543".to_string(),
            date: "2026-11-06".to_string(),
            time: "10:00:00".to_string(),
            duration_ms: index as u64,
        }
    }

    #[test]
    fn entry_uses_request_and_report_values() {
        let completed_at = DateTime::parse_from_rfc3339("2026-11-06T14:35:09+00:00")
            .expect("date")
            .with_timezone(&Local);
        let entry = RestoreHistoryEntry::from_restore(&request(), &report(), completed_at);

        assert_eq!(entry.file_name, "shop.sql");
        assert_eq!(entry.database, "shop");
        assert_eq!(entry.connection, "admin@db.local:6543");
        assert_eq!(entry.date, completed_at.format("%Y-%m-%d").to_string());
        assert_eq!(entry.time, completed_at.format("%H:%M:%S").to_string());
        assert_eq!(entry.duration_ms, 1500);
    }

    #[test]
    fn push_keeps_the_newest_entries() {
        let mut entries = Vec::new();
        for index in 0..=MAX_HISTORY_ENTRIES {
            push_entry(&mut entries, entry(index));
        }

        assert_eq!(entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(entries[0].file_name, "backup-1.sql");
        assert_eq!(
            entries[MAX_HISTORY_ENTRIES - 1].file_name,
            format!("backup-{MAX_HISTORY_ENTRIES}.sql")
        );
    }

    #[test]
    fn push_below_the_limit_keeps_every_entry() {
        let mut entries = Vec::new();
        for index in 0..MAX_HISTORY_ENTRIES {
            push_entry(&mut entries, entry(index));
        }
        assert_eq!(entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(entries[0].file_name, "backup-0.sql");
    }
}
