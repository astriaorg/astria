use std::sync::Arc;

use astria_sequencer::{
    accounts::types::{
        Address as SequencerAddress,
        Nonce,
    },
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction,
        Signed as SignedSequencerTx,
        Unsigned as UnsignedSequencerTx,
    },
};
use ed25519_consensus::SigningKey;
use ethers::types::Transaction;
use tokio::sync::mpsc::{
    Receiver,
    Sender,
};
use tracing::{
    error,
    trace,
    warn,
};

use super::{
    Action,
    Event,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid sequencer address: {0}")]
    InvalidSequencerAddress(String),
}

/// Struct for bundling transactions into sequencer txs.
// TODO: configure as "train with capacity", i.e. max number of txs to bundle and sequencer block
// time
pub struct Bundler {
    sequencer_address: SequencerAddress,
    rollup_chain_id: String,
}

impl Bundler {
    pub(super) fn new(sequencer_addr: String, rollup_chain_id: String) -> Result<Self, Error> {
        let sequencer_address = SequencerAddress::try_from_str(&sequencer_addr)
            .map_err(|_| Error::InvalidSequencerAddress(sequencer_addr))?;
        Ok(Self {
            sequencer_address,
            rollup_chain_id,
        })
    }

    /// Runs the Bundler service, listening for new transactions from the event channel,
    /// bundling them into sequencer txs and sending to the action channel.
    pub(super) async fn run(
        mut self,
        mut event_rx: Receiver<Event>,
        action_tx: Sender<Action>,
    ) -> Result<(), Error> {
        // grab event
        loop {
            if let Some(event) = event_rx.recv().await {
                if let Some(action) = self.process_event(event.clone()).await {
                    trace!(event=?event, action=?action, "Processed event into action");
                    match action_tx.send(action).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!(error=?e, "sending action failed");
                            todo!("handle action send failure");
                        }
                    }
                }
            }
        }
    }

    /// Processes an event to produce an action.
    async fn process_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::NewRollupTx(tx) => self
                .handle_new_tx(tx)
                .await
                .map(|sequencer_tx| Action::SendSequencerTx(sequencer_tx)),
        }
    }

    /// Handles a new transaction event by serializing it into a sequencer tx.
    ///
    /// # Errors
    ///
    /// - `Error::GetNonceFailed` if getting the nonce from the sequencer fails (only when nonce is
    ///   None)
    async fn handle_new_tx(&mut self, tx: Transaction) -> Option<SignedSequencerTx> {
        let chain_id = self.rollup_chain_id.clone();
        let rollup_tx = tx.clone();

        let signing_handle = tokio::task::spawn_blocking(move || {
            // For now, each transaction is transmitted from a new account with nonce 0
            let sequencer_key = SigningKey::new(rand::thread_rng());
            // serialize and pack into sequencer tx
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));

            let nonce = Nonce::from(0);

            UnsignedSequencerTx::new_with_actions(nonce, vec![seq_action])
                .into_signed(&sequencer_key)
        });
        let sequencer_tx = signing_handle
            .await
            .map_err(|e| warn!(error=?e, tx=?tx, "Transaction serialization and bundling failed"))
            .ok();

        sequencer_tx
    }
}
