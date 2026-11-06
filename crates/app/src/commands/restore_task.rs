//! Background restore task and its events.

use sqlrestore_core::application::commands::{
    CancelRestoreCommand, CancelRestoreCommandHandler, RestoreCommand, RestoreCommandHandler,
};
use sqlrestore_core::domain::error::RestoreError;
use sqlrestore_core::domain::restore::{RestoreReport, RestoreRequest};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// Messages sent from the restore thread to the interface.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreEvent {
    Progress(f32),
    Finished(RestoreOutcome),
}

/// Final result of a restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreOutcome {
    Success(RestoreReport),
    Cancelled,
    Failed(String),
}

/// Handle for a restore running outside the interface thread.
pub struct RestoreTask {
    receiver: Receiver<RestoreEvent>,
    cancel_handler: Arc<CancelRestoreCommandHandler>,
}

impl RestoreTask {
    /// Returns the next event without blocking.
    pub fn try_recv(&self) -> Result<Option<RestoreEvent>, TryRecvError> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
        }
    }

    /// Requests cancellation from the database and the loop.
    pub fn cancel(&self) -> Result<(), RestoreError> {
        self.cancel_handler.execute(CancelRestoreCommand)
    }
}

pub fn spawn_restore(
    handler: Arc<RestoreCommandHandler>,
    cancel_handler: Arc<CancelRestoreCommandHandler>,
    request: RestoreRequest,
) -> RestoreTask {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let progress_sender = sender.clone();
        let result = handler.execute(RestoreCommand { request }, &move |progress| {
            let _ = progress_sender.send(RestoreEvent::Progress(progress.fraction()));
        });

        let outcome = match result {
            Ok(report) => RestoreOutcome::Success(report),
            Err(RestoreError::Cancelled) => RestoreOutcome::Cancelled,
            Err(error) => RestoreOutcome::Failed(error.to_string()),
        };
        let _ = sender.send(RestoreEvent::Finished(outcome));
    });

    RestoreTask {
        receiver,
        cancel_handler,
    }
}
