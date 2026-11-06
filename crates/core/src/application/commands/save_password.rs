//! Save password command.

use std::sync::Arc;

use crate::domain::config::ConnectionSettings;
use crate::domain::error::PasswordError;
use crate::ports::password_store::PasswordStore;

/// User action: save the connection password.
pub struct SavePasswordCommand {
    pub connection: ConnectionSettings,
    pub password: String,
}

/// Handles [`SavePasswordCommand`].
pub struct SavePasswordCommandHandler {
    store: Arc<dyn PasswordStore>,
}

impl SavePasswordCommandHandler {
    pub fn new(store: Arc<dyn PasswordStore>) -> Self {
        Self { store }
    }

    pub fn execute(&self, command: SavePasswordCommand) -> Result<(), PasswordError> {
        self.store.save(&command.connection, &command.password)
    }
}
