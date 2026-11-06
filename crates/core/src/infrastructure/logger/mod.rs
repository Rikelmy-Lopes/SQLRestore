//! Logging adapters.

mod file_logger;
mod stderr_logger;
mod tee_logger;

pub use file_logger::FileLogger;
pub use stderr_logger::StderrLogger;
pub use tee_logger::TeeLogger;
