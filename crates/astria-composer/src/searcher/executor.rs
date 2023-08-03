use astria_sequencer::transaction::Signed as SignedSequencerTx;
use astria_sequencer_client::Client as SequencerClient;
use color_eyre::eyre;
use ed25519_consensus::SigningKey;
use tokio::sync::mpsc::Receiver;
use tracing::{
    error,
    trace,
};

use super::Action;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sequencer tx submission failed")]
    TxSubmissionFailed,
}

/// Struct for executing sequencer actions
pub struct SequencerExecutor {
    sequencer_client: SequencerClient,
    sequencer_key: SigningKey,
}

impl SequencerExecutor {
    pub(super) fn new(sequencer_url: String, secret: &str) -> eyre::Result<Self> {
        // TODO: should be tendermint_rpc::HttpClient
        let sequencer_client = SequencerClient::new(&sequencer_url)?;
        let secret_bytes: [u8; 32] = hex::decode(secret).unwrap().try_into().unwrap();
        let sequencer_key = SigningKey::from(secret_bytes);
        Ok(Self {
            sequencer_client,
            sequencer_key,
        })
    }

    /// Run the executor, listening for new actions from the actions channel and processing them,
    /// e.g. by submitting sequencer txs.
    ///
    /// # Errors
    ///
    /// - `Error::ActionRecv` if receiving an action from the channel fails
    /// - `Error::TxSubmissionFailed` if the sequencer tx submission fails
    pub(super) async fn run(self, mut action_rx: Receiver<Action>) -> Result<(), Error> {
        loop {
            if let Some(action) = action_rx.recv().await {
                self.process_action(action).await?;
            }
        }
    }

    /// Process an action.
    async fn process_action(&self, action: Action) -> Result<(), Error> {
        match action {
            Action::SendSequencerTx(tx) => self.handle_send_sequencer_tx(tx).await,
        }
    }

    /// Handle a `SendSequencerSecondaryTx` action.
    ///
    /// # Errors
    ///
    /// - `Error::TxSubmissionFailed` if the sequencer tx submission fails
    async fn handle_send_sequencer_tx(&self, tx: SignedSequencerTx) -> Result<(), Error> {
        let submission_response = self
            .sequencer_client
            .submit_transaction_sync(tx.clone())
            .await
            .map_err(|_| {
                error!(tx=?tx, "sequencer tx submission failed");
                Error::TxSubmissionFailed
            })?;

        // TODO: is there more error checking that should be done on this?
        if submission_response.code != 0.into() {
            error!(tx=?tx, "sequencer tx submission failed");
            return Err(Error::TxSubmissionFailed);
        } else {
            trace!(tx=?tx, "sequencer tx submitted")
        }
        Ok(())
    }
}
