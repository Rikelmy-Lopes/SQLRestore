//! Logger that writes to standard error.

use chrono::Local;

use crate::ports::logger::{LogLevel, Logger};

/// Writes log entries to the terminal. Used in debug builds.
pub struct StderrLogger;

impl Logger for StderrLogger {
    fn log(&self, level: LogLevel, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        eprintln!("{timestamp} [{}] {message}", level.as_str());
    }
}
