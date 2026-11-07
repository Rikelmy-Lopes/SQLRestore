//! Background connection test task.

use sqlrestore_core::application::commands::{TestConnectionCommand, TestConnectionCommandHandler};
use sqlrestore_core::domain::config::ConnectionSettings;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Handle for a connection test running outside the interface thread.
pub struct ConnectionTestTask {
    receiver: Receiver<Result<(), String>>,
}

impl ConnectionTestTask {
    /// Returns the result without blocking.
    pub fn try_recv(&self) -> Result<Option<Result<(), String>>, TryRecvError> {
        match self.receiver.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
        }
    }
}

pub fn spawn_connection_test(
    handler: Arc<TestConnectionCommandHandler>,
    connection: ConnectionSettings,
    password: String,
    recreate_database: bool,
) -> ConnectionTestTask {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = handler
            .execute(TestConnectionCommand {
                connection,
                password,
                recreate_database,
            })
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });

    ConnectionTestTask { receiver }
}
