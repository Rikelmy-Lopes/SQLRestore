//! Logger that forwards every entry to several loggers.

use std::sync::Arc;

use crate::ports::logger::{LogLevel, Logger};

/// Sends each entry to all wrapped loggers.
pub struct TeeLogger {
    loggers: Vec<Arc<dyn Logger>>,
}

impl TeeLogger {
    pub fn new(loggers: Vec<Arc<dyn Logger>>) -> Self {
        Self { loggers }
    }
}

impl Logger for TeeLogger {
    fn log(&self, level: LogLevel, message: &str) {
        for logger in &self.loggers {
            logger.log(level, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryLogger {
        entries: Mutex<Vec<(LogLevel, String)>>,
    }

    impl Logger for MemoryLogger {
        fn log(&self, level: LogLevel, message: &str) {
            self.entries
                .lock()
                .expect("logger lock")
                .push((level, message.to_string()));
        }
    }

    #[test]
    fn forwards_to_every_logger() {
        let first = Arc::new(MemoryLogger::default());
        let second = Arc::new(MemoryLogger::default());
        let logger = TeeLogger::new(vec![first.clone(), second.clone()]);

        logger.log(LogLevel::Warning, "message");

        for memory in [first, second] {
            let entries = memory.entries.lock().expect("logger lock");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].0, LogLevel::Warning);
            assert_eq!(entries[0].1, "message");
        }
    }
}
