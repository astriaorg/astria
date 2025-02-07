use std::{
    fmt::Display,
    sync::Arc,
};

use astria_core::{
    sequencerblock,
    sequencerblock::v1::block,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    WrapErr as _,
};
use futures::{
    Future,
    FutureExt as _,
};
use sequencer_client::tendermint_rpc::endpoint::broadcast::tx_sync;
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{
    bid::{
        Bid,
        RollupBlockHash,
    },
    sequencer_key::SequencerKey,
};

pub(super) mod factory;
pub(super) use factory::Factory;
mod allocation_rule;
mod worker;
use worker::Worker;

/// Used to uniquely identify an auction.
///
/// Currently the same as the proposed sequencer block.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(super) struct Id([u8; 32]);

impl Id {
    pub(super) fn from_sequencer_block_hash(block_hash: &block::Hash) -> Self {
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

/// The frontend to interact with a running auction.
pub(super) struct Auction {
    /// The idenfifier of the current auction.
    id: Id,
    /// The block hash of the proposed Sequencer block that triggered the creation of this auction.
    block_hash: block::Hash,
    /// The height of the proposed Sequencer block that triggered this auction.
    height: u64,
    /// The hash of the rollup block that was executed and on which all bids will based.
    hash_of_executed_block_on_rollup: Option<RollupBlockHash>,
    /// A oneshot channel to trigger the running auction to start accepting bids.
    start_bids: Option<oneshot::Sender<()>>,
    /// A oneshot channel to trigger the running auction to start its auction timer.
    start_timer: Option<oneshot::Sender<()>>,
    /// A channel to forward bids from Auctioneer's stream connected to its Rollup to the
    /// background auction task.
    bids: mpsc::UnboundedSender<Arc<Bid>>,
    /// Used to cancel the worker task.
    cancellation_token: CancellationToken,
    /// The actual event loop running in the background that receives bids, times the
    /// auction, and submits the winner to Sequencer.
    worker: JoinHandle<Result<Summary, worker::Error>>,
    metrics: &'static crate::Metrics,
    started_at: std::time::Instant,
}

impl Auction {
    pub(super) fn abort(&self) {
        self.worker.abort();
    }

    pub(super) fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub(in crate::auctioneer) fn id(&self) -> &Id {
        &self.id
    }

    // TODO: identify the commitment in span fields
    #[instrument(skip_all, fields(id = %self.id), err)]
    pub(super) fn start_timer(
        &mut self,
        commitment: sequencerblock::optimistic::v1alpha1::SequencerBlockCommit,
    ) -> eyre::Result<()> {
        ensure!(
            &self.block_hash == commitment.block_hash() && self.height == commitment.height(),
            "commitment does not match auction; auction.block_hash = `{}`, auction.height = `{}`, \
             commitment.block_hash = `{}`, commitment.height = `{}`",
            self.block_hash,
            self.height,
            commitment.block_hash(),
            commitment.height(),
        );
        if let Some(start_timer) = self.start_timer.take() {
            start_timer
                .send(())
                .map_err(|()| eyre!("the auction worker's start timer channel was already dropped"))
        } else {
            Err(eyre!(
                "a previous commitment already triggered the start timer of the auction"
            ))
        }
    }

    // TODO: identify the executed block in the span fields
    #[instrument(skip_all, fields(id = %self.id), err)]
    pub(in crate::auctioneer) fn start_bids(
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

        if let Some(start_bids) = self.start_bids.take() {
            start_bids.send(()).map_err(|()| {
                eyre!("the auction worker's start bids channel was already dropped")
            })?;
        } else {
            bail!("a previous executed block already triggered the auction to start bids");
        }

        let prev_block = self
            .hash_of_executed_block_on_rollup
            .replace(block.rollup_block_hash());
        debug_assert!(prev_block.is_none());

        Ok(())
    }

    // TODO: Use a refinement type for the parente rollup block hash
    #[instrument(skip_all, fields(
        id = %self.id,
        bid.sequencer_block_hash = %bid.sequencer_parent_block_hash(),
        bid.parent_roll_block_hash = %bid.rollup_parent_block_hash(),
    ), err)]
    pub(in crate::auctioneer) fn forward_bid_to_auction(
        &mut self,
        bid: Arc<Bid>,
    ) -> eyre::Result<()> {
        if let err @ Err(_) = self.does_bid_match_auction(&bid) {
            self.metrics
                .increment_auction_bids_without_matching_auction();
            return err;
        }

        self.metrics
            .record_auction_bid_delay_since_start(self.started_at.elapsed());

        self.bids
            .send(bid)
            .wrap_err("failed to submit bid to auction; the bid is lost")
    }

    fn does_bid_match_auction(&self, bid: &Bid) -> eyre::Result<()> {
        ensure!(
            &self.block_hash == bid.sequencer_parent_block_hash()
                && self.hash_of_executed_block_on_rollup.as_ref()
                    == Some(bid.rollup_parent_block_hash()),
            "bid does not match auction; auction.sequencer_parent_block_hash = `{}`, \
             auction.rollup_parent_block_hash = `{}`, bid.sequencer_parent_block_hash = `{}`, \
             bid.rollup_parent_block_hash = `{}`",
            self.block_hash,
            fmt_none_as_msg(
                self.hash_of_executed_block_on_rollup.as_ref(),
                "<not yet received>"
            ),
            bid.sequencer_parent_block_hash(),
            bid.rollup_parent_block_hash(),
        );
        Ok(())
    }
}

fn fmt_none_as_msg<'a, T: Display>(
    val: Option<&'a T>,
    default: &'static str,
) -> FmtNoneAsMsg<'a, T> {
    FmtNoneAsMsg {
        val,
        default,
    }
}

/// Utilty to implement the [`Display`] trait on a type `Option<T>`.
///
/// Writes `default` if `val` is `None`, or the set value if `Some`.
struct FmtNoneAsMsg<'a, T> {
    val: Option<&'a T>,
    default: &'static str,
}
impl<T> Display for FmtNoneAsMsg<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.val {
            Some(val) => val.fmt(f),
            None => f.write_str(self.default),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("the task running the auction panicked")]
    Panicked { source: tokio::task::JoinError },
    #[error("the auction failed")]
    Failed { source: worker::Error },
}

impl Future for Auction {
    type Output = (Id, Result<Summary, Error>);

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = match std::task::ready!(self.worker.poll_unpin(cx)) {
            Ok(Ok(summary)) => Ok(summary),
            Ok(Err(source)) => Err(Error::Failed {
                source,
            }),
            Err(source) => Err(Error::Panicked {
                source,
            }),
        };
        std::task::Poll::Ready((self.id, res))
    }
}

pub(super) enum Summary {
    CancelledDuringAuction,
    NoBids,
    Submitted {
        response: tx_sync::Response,
        nonce_used: u32,
    },
}

impl Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Summary::CancelledDuringAuction => {
                f.write_str("received cancellation signal during auction loop")
            }
            Summary::NoBids => f.write_str("auction finished without bids"),
            Summary::Submitted {
                response,
                nonce_used,
            } => write!(
                f,
                "auction winner submitted using nonce `{nonce_used}`; Sequencer responded with \
                 ABCI code `{}`, log `{}`",
                response.code.value(),
                response.log,
            ),
        }
    }
}
