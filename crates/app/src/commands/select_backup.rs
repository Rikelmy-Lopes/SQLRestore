//! File selection command.

use std::path::PathBuf;

/// User action: choose a `.sql` backup file.
pub struct SelectBackupCommandHandler;

impl SelectBackupCommandHandler {
    /// Opens the native file dialog. Returns `None` when cancelled.
    pub fn execute(&self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("SQL", &["sql"])
            .pick_file()
    }
}
