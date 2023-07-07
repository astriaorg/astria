use astria_sequencer::accounts::types::Nonce;
use tokio::sync::broadcast::{
    error::RecvError,
    Receiver,
    Sender,
};
use tracing::error;

use super::Action;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("receiving action failed")]
    ActionRecv(#[source] RecvError),
    #[error("invalid nonce")]
    InvalidNonce(Nonce),
}

pub struct SequencerExecutor();

impl SequencerExecutor {
    pub(super) fn new() -> Self {
        Self()
    }

    pub(super) async fn run(self, mut action_rx: Receiver<Action>) -> Result<(), Error> {
        loop {
            match action_rx.recv().await {
                Ok(action) => {
                    Self::process_action(action).await;
                }
                Err(e) => {
                    error!(error=?e, "receiving action failed");
                    // todo!("kill the executor?");
                    return Err(Error::ActionRecv(e));
                }
            }
        }
    }

    async fn process_action(action: Action) {
        match action {
            Action::SendSequencerSecondaryTx(_tx) => {
                todo!("sign tx and send to sequencer")
            }
        }
    }
}
