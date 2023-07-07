use std::sync::Arc;

use astria_sequencer::accounts::types::Nonce;
use astria_sequencer_client::Client as SequencerClient;
use ed25519_consensus::SigningKey;
use tokio::sync::broadcast::{
    error::RecvError,
    Receiver,
};
use tracing::error;

use super::Action;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("receiving action failed")]
    ActionRecv(#[source] RecvError),
    #[error("invalid nonce")]
    InvalidNonce(Nonce),
    #[error("sequencer signer init failed")]
    SequencerSignerInit(#[source] ed25519_consensus::Error),
    #[error("sequencer tx submission failed")]
    TxSubmissionFailed,
}

pub struct SequencerExecutor {
    sequencer_client: Arc<SequencerClient>,
    // TODO: add sequencer key
    sequencer_key: SigningKey,
}

impl SequencerExecutor {
    pub(super) fn new(sequencer_client: Arc<SequencerClient>, secret: &str) -> Self {
        let secret_bytes: [u8; 32] = hex::decode(secret).unwrap().try_into().unwrap();
        let sequencer_key = SigningKey::from(secret_bytes);
        Self {
            sequencer_client,
            sequencer_key,
        }
    }

    pub(super) async fn run(self, mut action_rx: Receiver<Action>) -> Result<(), Error> {
        loop {
            match action_rx.recv().await {
                Ok(action) => {
                    self.process_action(action).await?;
                }
                Err(e) => {
                    error!(error=?e, "receiving action failed");
                    // todo!("kill the executor?");
                    return Err(Error::ActionRecv(e));
                }
            }
        }
    }

    async fn process_action(&self, action: Action) -> Result<(), Error> {
        match action {
            Action::SendSequencerSecondaryTx(tx) => {
                let signed = tx.into_signed(&self.sequencer_key);
                let submission_response = self
                    .sequencer_client
                    .submit_transaction_sync(signed.clone())
                    .await
                    .map_err(|_| {
                        error!(tx=?signed, "sequencer tx submission failed");
                        Error::TxSubmissionFailed
                    })?;

                // TODO: is there more error checking that should be done on this?
                if submission_response.code != 0.into() {
                    error!(tx=?signed, "sequencer tx submission failed");
                    return Err(Error::TxSubmissionFailed);
                }
                Ok(())
            }
        }
    }
}
