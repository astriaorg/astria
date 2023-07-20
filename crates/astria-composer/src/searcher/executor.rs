use astria_sequencer::transaction::Unsigned;
use astria_sequencer_client::Client;
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use ed25519_consensus::SigningKey;
use tokio::sync::broadcast::{
    error::RecvError,
    Receiver,
};
use tracing::{
    trace,
    warn,
};

use super::Action;

/// Struct for executing sequencer actions
// #[derive(Debug)]
pub struct SequencerExecutor {
    sequencer_client: Client,
    sequencer_key: SigningKey,
}

impl SequencerExecutor {
    pub(super) fn new(sequencer_client: Client, secret: &str) -> Self {
        let secret_bytes: [u8; 32] = hex::decode(secret).unwrap().try_into().unwrap();
        let sequencer_key = SigningKey::from(secret_bytes);
        Self {
            sequencer_client,
            sequencer_key,
        }
    }

    /// Run the executor, listening for new actions from the actions channel and processing them,
    /// e.g. by submitting sequencer txs.
    ///
    /// # Errors
    ///
    /// - `Error::ActionRecv` if receiving an action from the channel fails
    /// - `Error::TxSubmissionFailed` if the sequencer tx submission fails
    pub(super) async fn run(self, mut action_rx: Receiver<Action>) -> eyre::Result<()> {
        loop {
            match action_rx.recv().await {
                Ok(action) => {
                    self.process_action(action).await?;
                }
                Err(RecvError::Lagged(messages_skipped)) => {
                    warn!(
                        messages_skipped,
                        "action broadcast receiver is lagging behind"
                    );
                }
                Err(RecvError::Closed) => bail!("broadcast channel closed unexpectedly"),
            }
        }
    }

    /// Process an action.
    ///
    /// # Errors
    ///
    /// - `Error::TxSubmissionFailed` if the sequencer tx submission fails
    async fn process_action(&self, action: Action) -> eyre::Result<()> {
        match action {
            Action::SendSequencerSecondaryTx(tx) => self
                .handle_send_sequencer_secondary_tx(tx)
                .await
                .wrap_err("failed sending sequencers secondary tx"),
        }
    }

    /// Handle a `SendSequencerSecondaryTx` action.
    ///
    /// # Errors
    ///
    /// - `Error::TxSubmissionFailed` if the sequencer tx submission fails
    async fn handle_send_sequencer_secondary_tx(&self, tx: Unsigned) -> eyre::Result<()> {
        let signed = tx.into_signed(&self.sequencer_key);
        let submission_response = self
            .sequencer_client
            .submit_transaction_sync(signed.clone())
            .await
            .wrap_err("failed to submit transaction to sequencer")?;

        // TODO: is there more error checking that should be done on this?
        ensure!(
            submission_response.code.is_ok(),
            "sequencer responded with non zero code",
        );
        trace!(tx=?signed, "sequencer tx submitted");
        Ok(())
    }
}
