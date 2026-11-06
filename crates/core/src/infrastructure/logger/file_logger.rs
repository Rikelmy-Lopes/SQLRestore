//! File logger with size-based rotation.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Local;

use crate::infrastructure::paths;
use crate::ports::logger::{LogLevel, Logger};

const DEFAULT_MAX_BYTES: u64 = 5 * 1024 * 1024;
const DEFAULT_KEEP_OLD: usize = 3;

/// Writes log entries to a file and rotates it when it grows too large.
pub struct FileLogger {
    path: PathBuf,
    max_bytes: u64,
    keep_old: usize,
    file: Mutex<Option<File>>,
}

impl FileLogger {
    /// Logger at the platform AppData location with the default limits.
    pub fn default_location() -> Self {
        Self::new(paths::log_path())
    }

    /// Logger at the given path with the default limits.
    pub fn new(path: PathBuf) -> Self {
        Self::with_limits(path, DEFAULT_MAX_BYTES, DEFAULT_KEEP_OLD)
    }

    /// Logger with custom limits. Used by tests.
    pub fn with_limits(path: PathBuf, max_bytes: u64, keep_old: usize) -> Self {
        Self {
            path,
            max_bytes,
            keep_old,
            file: Mutex::new(None),
        }
    }

    fn open(&self) -> Option<File> {
        if let Some(parent) = self.path.parent()
            && fs::create_dir_all(parent).is_err()
        {
            return None;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok()
    }

    fn rotate(&self) {
        if self.keep_old == 0 {
            let _ = fs::remove_file(&self.path);
            return;
        }

        let _ = fs::remove_file(rotated_path(&self.path, self.keep_old));
        for index in (1..self.keep_old).rev() {
            let from = rotated_path(&self.path, index);
            if from.exists() {
                let to = rotated_path(&self.path, index + 1);
                let _ = fs::remove_file(&to);
                let _ = fs::rename(&from, &to);
            }
        }
        let first = rotated_path(&self.path, 1);
        let _ = fs::remove_file(&first);
        let _ = fs::rename(&self.path, &first);
    }

    fn write_entry(&self, level: LogLevel, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("{timestamp} [{}] {message}\n", level.as_str());

        let Ok(mut guard) = self.file.lock() else {
            return;
        };

        let current_size = fs::metadata(&self.path).map_or(0, |metadata| metadata.len());
        if current_size.saturating_add(line.len() as u64) > self.max_bytes {
            // The open file must be closed before it can be renamed on Windows.
            *guard = None;
            self.rotate();
        }

        if guard.is_none() {
            *guard = self.open();
        }

        if let Some(file) = guard.as_mut() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

impl Logger for FileLogger {
    fn log(&self, level: LogLevel, message: &str) {
        self.write_entry(level, message);
    }
}

fn rotated_path(path: &Path, index: usize) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{index}"));
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_entries() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("logs").join("test.log");
        let logger = FileLogger::new(path.clone());
        logger.log(LogLevel::Info, "first");
        logger.log(LogLevel::Error, "second");

        let text = fs::read_to_string(path).expect("log file");
        assert!(text.contains("[INFO] first"));
        assert!(text.contains("[ERROR] second"));
    }

    #[test]
    fn rotates_and_keeps_requested_files() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("test.log");
        let logger = FileLogger::with_limits(path.clone(), 80, 2);

        for index in 0..10 {
            logger.log(LogLevel::Info, &format!("message number {index}"));
        }

        assert!(path.is_file());
        assert!(rotated_path(&path, 1).is_file());
        assert!(rotated_path(&path, 2).is_file());
        assert!(!rotated_path(&path, 3).is_file());
        assert_eq!(fs::read_dir(directory.path()).expect("entries").count(), 3);
    }

    #[test]
    fn unwritable_path_does_not_panic() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let blocker = directory.path().join("blocker");
        fs::write(&blocker, "file").expect("write");
        let logger = FileLogger::new(blocker.join("test.log"));
        logger.log(LogLevel::Error, "must not panic");
    }
}
