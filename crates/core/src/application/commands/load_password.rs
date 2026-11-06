//! Load password command.

use std::sync::Arc;

use crate::domain::config::ConnectionSettings;
use crate::domain::error::PasswordError;
use crate::ports::password_store::PasswordStore;

/// User action: read the saved password.
pub struct LoadPasswordCommand {
    pub connection: ConnectionSettings,
}

/// Handles [`LoadPasswordCommand`].
pub struct LoadPasswordCommandHandler {
    store: Arc<dyn PasswordStore>,
}

impl LoadPasswordCommandHandler {
    pub fn new(store: Arc<dyn PasswordStore>) -> Self {
        Self { store }
    }

    pub fn execute(&self, command: LoadPasswordCommand) -> Result<Option<String>, PasswordError> {
        self.store.load(&command.connection)
    }
}
