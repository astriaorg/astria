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

use astria_core::{
    self,
    sequencerblock::v1::block::BlockHash,
};
use astria_eyre::eyre::{
    self,
    ensure,
    Context,
};
use futures::{
    Future,
    FutureExt as _,
};
use telemetry::display::base64;
use tokio::{
    sync::mpsc,
    task::JoinHandle,
};
use tracing::instrument;

use super::PendingNonceSubscriber;
use crate::{
    block::Commitment,
    bundle::Bundle,
    flatten_join_result,
    sequencer_key::SequencerKey,
};

pub(super) mod factory;
pub(super) use factory::Factory;
mod allocation_rule;
mod worker;
use worker::Worker;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(super) struct Id([u8; 32]);

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

pub(super) struct Auction {
    id: Id,
    block_hash: BlockHash,
    height: u64,
    parent_block_of_executed: Option<[u8; 32]>,
    commands: mpsc::Sender<Command>,
    bundles: mpsc::Sender<Bundle>,
    worker: JoinHandle<eyre::Result<()>>,
}

impl Auction {
    pub(super) fn abort(&self) {
        self.worker.abort();
    }

    pub(in crate::auctioneer::inner) fn id(&self) -> &Id {
        &self.id
    }

    #[instrument(skip_all, fields(id = %self.id), err)]
    pub(super) fn start_timer(&mut self, commitment: Commitment) -> eyre::Result<()> {
        ensure!(
            &self.block_hash == commitment.block_hash() && self.height == commitment.height(),
            "commitment does not match auction; auction.block_hash = `{}`, auction.height = `{}`, \
             commitment.block_hash = `{}`, commitment.height = `{}`",
            self.block_hash,
            self.height,
            commitment.block_hash(),
            commitment.height(),
        );
        self.commands
            .try_send(Command::StartTimer)
            .wrap_err("failed to send command to start timer to auction")
    }

    #[instrument(skip_all, fields(id = %self.id), err)]
    pub(in crate::auctioneer::inner) fn start_processing_bids(
        &mut self,
        block: crate::block::Executed,
    ) -> eyre::Result<()> {
        ensure!(
            &self.block_hash == block.sequencer_block_hash(),
            "executed block does not match auction; auction.block_hash = `{}`, \
             executed.block_hash = `{}`",
            &self.block_hash,
            block.sequencer_block_hash(),
        );
        // TODO: What if it was already set? Overwrite? Replace? Drop?
        let _ = self
            .parent_block_of_executed
            .replace(block.parent_rollup_block_hash());
        self.commands
            .try_send(Command::StartProcessingBids)
            .wrap_err("failed to send command to start processing bids")
    }

    // TODO: Use a refinement type for the parente rollup block hash
    #[instrument(skip_all, fields(
        id = %self.id,
        bundle.sequencer_block_hash = %bundle.base_sequencer_block_hash(),
        bundle.parent_roll_block_hash = %base64(bundle.parent_rollup_block_hash()),

    ), err)]
    pub(in crate::auctioneer::inner) fn forward_bundle_to_auction(
        &mut self,
        bundle: Bundle,
    ) -> eyre::Result<()> {
        // TODO: emit some more information about auctoin ID, expected vs actual parent block hash,
        // tacked block hash, provided block hash, etc.
        let Some(parent_block_of_executed) = self.parent_block_of_executed else {
            eyre::bail!(
                "received a new bundle but the current auction has not yet
                    received an execute block from the rollup; dropping the bundle"
            );
        };
        ensure!(
            &self.block_hash == bundle.base_sequencer_block_hash()
                && parent_block_of_executed == bundle.parent_rollup_block_hash(),
            "bundle does not match auction; auction.sequenecer_block_hash = `{}`, \
             auction.parent_block_hash = `{}`, bundle. = `{}`, bundle.height = `{}`",
            self.block_hash,
            base64(parent_block_of_executed),
            bundle.base_sequencer_block_hash(),
            base64(bundle.parent_rollup_block_hash()),
        );
        self.bundles
            .try_send(bundle)
            .wrap_err("failed to submit bundle to auction")
    }
}

impl Future for Auction {
    type Output = (Id, eyre::Result<()>);

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = std::task::ready!(self.worker.poll_unpin(cx));
        std::task::Poll::Ready((self.id, flatten_join_result(res)))
    }
}
