//! The Auction is repsonsible for running an auction for a given block. An auction advances through
//! the following states, controlled via the `commands_rx` channel received:
//! 1. The auction is initialized but not yet started (i.e. no commands have been received).
//! 2. After receiving a `Command::StartProcessingBids`, the auction will start processing incoming
//!    bundles from `new_bundles_rx`.
//! 3. After receiving a `Command::StartTimer`, the auction will set a timer for `latency_margin`
//!    (denominated in milliseconds).
//! 4. Once the timer expires, the auction will choose a winner based on its `AllocationRule` and
//!    submit it to the sequencer.
//!
//! ## Aborting an Auction
//! The auction may also be aborted at any point before the timer expires.
//! This will cause the auction to return early without submitting a winner,
//! effectively discarding any bundles that were processed.
//! This is used for leveraging optimsitic execution, running an auction for block data that has
//! been proposed in the sequencer network's cometBFT but not yet finalized.
//! We assume that most proposals are adopted in cometBFT, allowing us to buy a few hundred
//! milliseconds before they are finalized. However, if multiple rounds of voting invalidate a
//! proposal, we can abort the auction and avoid submitting a potentially invalid bundle. In this
//! case, the auction will abort and a new one will be created for the newly processed proposal
//! (which will be received by the Optimistic Executor via the optimistic block stream).
//!
//! ## Auction Result
//! The auction result is a `Bundle` that is signed by the Auctioneer and submitted to the rollup
//! via the sequencer. The rollup defines a trusted Auctioneer address that it allows to submit
//! bundles, and thus must verify the Auctioneer's signature over this bundle.
//!
//! Since the sequencer does not include the transaction signer's metadata with the `RollupData`
//! events that it saves in its block data, the Auctioneer must include this metadata in its
//! `RollupDataSubmission`s. This is done by wrapping the winning `Bundle` object in an
//! `AuctionResult` object, which is then serialized into the `RollupDataSubmission`.
//!
//! ## Submission to Sequencer
//! The auction will submit the winning bundle to the sequencer via the `broadcast_tx_sync` ABCI(?)
//! endpoint.
//! In order to save time on fetching a nonce, the auction will fetch the next pending nonce as soon
//! as it received the signal to start the timer. This corresponds to the sequencer block being
//! committed, thus providing the latest pending nonce.

use std::time::Duration;

use allocation_rule::FirstPrice;
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    sequencerblock::v1::block::BlockHash,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
    OptionExt as _,
};
pub(super) use builder::Builder;
use sequencer_client::SequencerClientExt;
use tokio::{
    select,
    sync::mpsc,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
};

use super::PendingNonceSubscriber;
use crate::{
    bundle::Bundle,
    sequencer_key::SequencerKey,
};

mod allocation_rule;
mod builder;
pub(super) mod factory;
mod running;
pub(super) use factory::Factory;
pub(super) use running::Running;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
struct Id([u8; 32]);

impl Id {
    pub(super) fn from_sequencer_block_hash(block_hash: &BlockHash) -> Self {
        Self(block_hash.get())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::STANDARD,
        };
        Base64Display::new(self.0.as_ref(), &STANDARD).fmt(f)
    }
}

enum Command {
    StartProcessingBids,
    StartTimer,
}

pub(super) struct Handle {
    commands_tx: mpsc::Sender<Command>,
    new_bundles_tx: mpsc::Sender<Bundle>,
}

impl Handle {
    pub(super) fn start_processing_bids(&mut self) -> eyre::Result<()> {
        self.commands_tx
            .try_send(Command::StartProcessingBids)
            .wrap_err("unable to send command to start processing bids to auction")?;
        Ok(())
    }

    pub(super) fn start_timer(&mut self) -> eyre::Result<()> {
        self.commands_tx
            .try_send(Command::StartTimer)
            .wrap_err("unable to send command to start time to auction")?;

        Ok(())
    }

    pub(super) fn try_send_bundle(&mut self, bundle: Bundle) -> eyre::Result<()> {
        self.new_bundles_tx
            .try_send(bundle)
            .wrap_err("bid channel full")?;

        Ok(())
    }
}

struct Auction {
    /// The sequencer's ABCI client, used for submitting transactions
    sequencer_abci_client: sequencer_client::HttpClient,
    /// Channel for receiving commands sent via the handle
    commands_rx: mpsc::Receiver<Command>,
    /// Channel for receiving new bundles
    new_bundles_rx: mpsc::Receiver<Bundle>,
    /// The time between receiving a block commitment
    latency_margin: Duration,
    /// The ID of the auction
    id: Id,
    /// The key used to sign transactions on the sequencer
    sequencer_key: SequencerKey,
    /// Fee asset for submitting transactions
    fee_asset_denomination: asset::Denom,
    /// The chain ID used for sequencer transactions
    sequencer_chain_id: String,
    /// Rollup ID to submit the auction result to
    rollup_id: RollupId,
    pending_nonce: PendingNonceSubscriber,
}

impl Auction {
    #[instrument(skip_all, fields(id = %self.id))]
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let mut latency_margin_timer = None;
        // TODO: do we want to make this configurable to allow for more complex allocation rules?
        let mut allocation_rule = FirstPrice::new();
        let mut auction_is_open = false;

        let auction_result = loop {
            select! {
                biased;

                // get the auction winner when the timer expires
                _ = async { latency_margin_timer.as_mut().unwrap() }, if latency_margin_timer.is_some() => {
                    break Ok(allocation_rule.winner());
                }

                Some(cmd) = self.commands_rx.recv() => {
                    match cmd {
                        Command::StartProcessingBids => {
                            if auction_is_open {
                                break Err(eyre!("auction received signal to start processing bids twice"));
                            }
                            auction_is_open = true;
                        },
                        Command::StartTimer  => {
                            if !auction_is_open {
                                break Err(eyre!("auction received signal to start timer before signal to start processing bids"));
                            }

                            // set the timer
                            latency_margin_timer = Some(tokio::time::sleep(self.latency_margin));
                        }
                    }
                }

                Some(bundle) = self.new_bundles_rx.recv(), if auction_is_open => {
                    if allocation_rule.bid(bundle.clone()) {
                        info!(
                            auction.id = %self.id,
                            bundle.bid = %bundle.bid(),
                            "received new highest bid"
                        );
                    } else {
                        debug!(
                            auction.id = %self.id,
                            bundle.bid = %bundle.bid(),
                            "received bid lower than current highest bid, discarding"
                        );
                    }
                }

            }
        };

        // TODO: report the pending nonce that we ended up using.
        let transaction_body = auction_result
            .wrap_err("auction failed unexpectedly")?
            .ok_or_eyre("auction ended with no winning bid")?
            .into_transaction_body(
                self.pending_nonce.get(),
                self.rollup_id,
                &self.sequencer_key,
                self.fee_asset_denomination.clone(),
                self.sequencer_chain_id,
            );

        let transaction = transaction_body.sign(self.sequencer_key.signing_key());

        // NOTE: Submit fire-and-forget style. If the submission didn't make it in time,
        // it's likey lost.
        // TODO: We can consider providing a very tight retry mechanism. Maybe resubmit once
        // if the response didn't take too long? But it's probably a bad idea to even try.
        // Can we detect if a submission failed due to a bad nonce? In this case, we could
        // immediately ("optimistically") submit with the most recent pending nonce (if the
        // publisher updated it in the meantime) or just nonce + 1 (if it didn't yet update)?
        match self
            .sequencer_abci_client
            .submit_transaction_sync(transaction)
            .await
            .wrap_err("submission of the auction failed; it's likely lost")
        {
            Ok(resp) => {
                // TODO: provide tx_sync response hash? Does it have extra meaning?
                info!(
                    response.log = %resp.log,
                    response.code = resp.code.value(),
                    "auction winner submitted to sequencer",
                );
            }
            Err(error) => {
                error!(%error, "failed to submit auction winner to sequencer; it's likely lost");
            }
        }
        Ok(())
    }
}
