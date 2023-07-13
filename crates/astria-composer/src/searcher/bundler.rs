use std::sync::Arc;

use astria_sequencer::{
    accounts::types::{
        Address as SequencerAddress,
        Nonce,
    },
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction,
        Unsigned,
    },
};
use astria_sequencer_client::Client as SequencerClient;
use ethers::types::Transaction;
use tokio::sync::broadcast::{
    error::{
        RecvError,
        SendError,
    },
    Receiver,
    Sender,
};
use tracing::{
    error,
    trace,
};

use super::{
    Action,
    Event,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid sequencer address: {0}")]
    InvalidSequencerAddress(String),
    #[error("failed to get sequencer nonce")]
    GetNonceFailed,
    #[error("receiving event failed")]
    EventRecv(#[source] RecvError),
    #[error("sending action failed")]
    ActionSend(#[source] SendError<Action>),
}

/// Struct for bundling transactions into sequencer txs.
// TODO: configure as "train with capacity", i.e. max number of txs to bundle and sequencer block
// time
pub struct Bundler {
    sequencer_client: Arc<SequencerClient>,
    sequencer_address: SequencerAddress,
    rollup_chain_id: String,
    current_nonce: Option<Nonce>,
}

impl Bundler {
    pub(super) fn new(
        sequencer_client: Arc<SequencerClient>,
        sequencer_addr: String,
        rollup_chain_id: String,
    ) -> Result<Self, Error> {
        let sequencer_address = SequencerAddress::try_from_str(&sequencer_addr)
            .map_err(|_| Error::InvalidSequencerAddress(sequencer_addr))?;
        Ok(Self {
            sequencer_client,
            sequencer_address,
            rollup_chain_id,
            current_nonce: None,
        })
    }

    /// Runs the Bundler service, listening for new transactions from the event channel,
    /// bundling them into sequencer txs and sending to the action channel.
    ///
    /// # Errors
    /// - `Error::EventRecv` if receiving an event from the event channel fails
    /// - `Error::ActionSend` if sending an action to the action channel fails
    /// - `Error::GetNonce` if getting the nonce from the sequencer fails
    pub(super) async fn run(
        mut self,
        mut event_rx: Receiver<Event>,
        action_tx: Sender<Action>,
    ) -> Result<(), Error> {
        // grab event
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let action = self.process_event(event).await?;
                    match action_tx.send(action.clone()) {
                        Ok(_) => trace!(action=?action, "action sent"),
                        Err(e) => {
                            error!(error=?e, "sending action failed");
                            // todo!("kill the bundler?");
                            return Err(Error::ActionSend(e));
                        }
                    }
                }
                Err(e) => {
                    error!(error=?e, "receiving event failed");
                    // todo!("kill the bundler?");
                    return Err(Error::EventRecv(e));
                }
            }
        }
    }

    /// Processes an event to produce an action.
    ///
    /// # Errors
    ///
    /// - `Error::GetNonceFailed` if getting the nonce from the sequencer fails
    async fn process_event(&mut self, event: Event) -> Result<Action, Error> {
        match event {
            Event::NewTx(tx) => self.handle_new_tx(tx).await,
        }
    }

    /// Handles a new transaction event by serializing it into a sequencer tx.
    ///
    /// # Errors
    ///
    /// - `Error::GetNonceFailed` if getting the nonce from the sequencer fails (only when nonce is
    ///   None)
    async fn handle_new_tx(&mut self, tx: Transaction) -> Result<Action, Error> {
        // serialize and pack into sequencer tx
        let data = tx.rlp().to_vec();
        let chain_id = self.rollup_chain_id.clone().into_bytes();
        // TODO: SequenceAction::new() needs to be pub
        let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));

        // get nonce
        let nonce = self.get_nonce().await?;

        let tx = Unsigned::new_with_actions(nonce, vec![seq_action]);
        // send action with sequencer tx
        Ok(Action::SendSequencerSecondaryTx(tx))
    }

    /// Gets the nonce from the sequencer. If the current nonce is nonce, fetches nonce from the
    /// sequencer, returning None if the request failed. Otherwise, increments the current nonce and
    /// returns it.
    ///
    /// # Errors
    /// Returns `Error::GetNonceFailed` if getting the nonce from the sequencer fails
    async fn get_nonce(&mut self) -> Result<Nonce, Error> {
        // get nonce if None otherwise increment it
        if let Some(nonce) = self.current_nonce {
            self.current_nonce = Some(nonce + Nonce::from(1));
        } else {
            self.current_nonce = self
                .sequencer_client
                .get_nonce(&self.sequencer_address, None)
                .await
                .ok();
        }
        self.current_nonce.ok_or(Error::GetNonceFailed)
    }

    /// Resets the nonce to None. Used from `Searcher::run()` if transaction execution fails
    /// because of nonce mismatch.
    pub(super) fn _reset_nonce(&mut self) {
        self.current_nonce = None;
    }
}
