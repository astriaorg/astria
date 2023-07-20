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
use astria_sequencer_client::Client;
use color_eyre::eyre::{
    self,
    bail,
    eyre,
    WrapErr as _,
};
use ethers::types::Transaction;
use tokio::sync::broadcast::{
    error::RecvError,
    Receiver,
    Sender,
};
use tracing::{
    instrument,
    trace,
    warn,
};

use super::{
    Action,
    Event,
};

/// Struct for bundling transactions into sequencer txs.
// TODO: configure as "train with capacity", i.e. max number of txs to bundle and sequencer block
// time
// #[derive(Debug)]
pub struct Bundler {
    sequencer_client: Client,
    sequencer_address: SequencerAddress,
    rollup_chain_id: String,
    current_nonce: Option<Nonce>,
}

impl Bundler {
    pub(super) fn new(
        sequencer_client: Client,
        sequencer_addr: String,
        rollup_chain_id: String,
    ) -> eyre::Result<Self> {
        let sequencer_address = SequencerAddress::try_from_str(&sequencer_addr)
            .map_err(|e| eyre!(Box::new(e)))
            .wrap_err("failed constructing sequencer address from string")?;
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
    #[instrument(name = "Bundler::run", skip_all)]
    pub(super) async fn run(
        mut self,
        mut event_rx: Receiver<Event>,
        action_tx: Sender<Action>,
    ) -> eyre::Result<()> {
        // grab event
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let action = self.process_event(event).await?;
                    match action_tx.send(action.clone()) {
                        Ok(_) => trace!(action=?action, "action sent"),
                        Err(e) => {
                            warn!(e.msg = %e, "broadcasting action failed");
                        }
                    }
                }
                Err(RecvError::Lagged(messages_skipped)) => {
                    warn!(
                        messages_skipped,
                        "event broadcast receiver is lagging behind"
                    );
                }
                Err(RecvError::Closed) => bail!("broadcast channel closed unexpectedly"),
            }
        }
    }

    /// Processes an event to produce an action.
    ///
    /// # Errors
    ///
    /// - `Error::GetNonceFailed` if getting the nonce from the sequencer fails
    async fn process_event(&mut self, event: Event) -> eyre::Result<Action> {
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
    async fn handle_new_tx(&mut self, tx: Transaction) -> eyre::Result<Action> {
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
    async fn get_nonce(&mut self) -> eyre::Result<Nonce> {
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
        self.current_nonce
            .ok_or_else(|| eyre!("failed getting current nonce"))
    }

    /// Resets the nonce to None. Used from `Searcher::run()` if transaction execution fails
    /// because of nonce mismatch.
    pub(super) fn _reset_nonce(&mut self) {
        self.current_nonce = None;
    }
}
