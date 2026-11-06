//! Logging port.

/// Severity of a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl LogLevel {
    /// Text written in the log file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Writes log entries. Implementations must never write secrets.
pub trait Logger: Send + Sync {
    /// Writes one entry. A failure to write is not fatal.
    fn log(&self, level: LogLevel, message: &str);
}
