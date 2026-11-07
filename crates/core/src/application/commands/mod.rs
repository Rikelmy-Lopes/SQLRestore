//! Commands invoked by the interface. Each command has one handler.

mod cancel_restore;
mod load_config;
mod load_password;
mod load_restore_history;
mod record_restore_history;
mod restore;
mod save_config;
mod save_password;
mod test_connection;

pub use cancel_restore::{CancelRestoreCommand, CancelRestoreCommandHandler};
pub use load_config::{LoadConfigCommand, LoadConfigCommandHandler};
pub use load_password::{LoadPasswordCommand, LoadPasswordCommandHandler};
pub use load_restore_history::{LoadRestoreHistoryCommand, LoadRestoreHistoryCommandHandler};
pub use record_restore_history::{RecordRestoreHistoryCommand, RecordRestoreHistoryCommandHandler};
pub use restore::{RestoreCommand, RestoreCommandHandler};
pub use save_config::{SaveConfigCommand, SaveConfigCommandHandler};
pub use save_password::{SavePasswordCommand, SavePasswordCommandHandler};
pub use test_connection::{TestConnectionCommand, TestConnectionCommandHandler};
