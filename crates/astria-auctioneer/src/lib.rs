//! Astria Auctioneer auctions bids for the top slot of a rollup's block.
//!
//! Auctioneer connects to a Sequencer node's
//! `astria.sequencerblock.optimistic.v1alpha1.OptimisticBlock`
//! gRPC interface, a Rollup's
//! `astria.auction.v1alpha1.OptimisticExecutionService`, and
//! a Rollup's `astria.auction.v1alpha.AuctionService`.
//!
//! # Starting an auction
//!
//! Every new proposed sequencer block (that is a block created
//! during Sequencer's CometBFT prepare and process proposal phase)
//! triggers Auctioneer to cancel a still running auction and start
//! a new one.
//!
//! Auctioneer forwards the block it received from Sequencer to its
//! the Rollup for (optimistic) execution, and then selects a winner
//! among the bids that are on top of this optimistically constructed
//! block. The winner is submitted to Sequencer to be included in
//! the next Sequencer block.
//!
//! # How a single auction works
//!
//! Once started, an auction waits for two signals:
//!
//! 1. one to open the auction for bids.
//! 2. the other to start the auction timer.
//!
//! The signal to open the auction for bids is usually given after
//! Auctioneer receives the executed block hash from its connected
//! rollup. Afterwards the running auction starts processing its received
//! bids given an allocation rule (right now first price only).
//!
//! The signal to start the auction timer is usually given after
//! Auctioneer receives a commit message from Sequencer. Once the
//! timer is up, the winner of the auction (if any) is submitted
//! to Sequencer.
//!
//! # Submitting an Auction to Sequencer
//!
//! An Auction is submitted to Sequencer using an ABCI
//! `broadcast_tx_sync` RPC. The payload is a regular
//! `astria.protocol.Tranasaction` signed by the auctioneer's
//! private ED25519 signing key.
//!
//! The moment an auction task starts its timer, it also requests
//! the latest nonce for auctioneer's account from Sequencer. If
//! Sequencer answers within the timer's duration, that nonce is
//! used to submit the winning allocation. If Sequencer does not
//! answer then the auction worker submits the winning bid using
//! the cached nonce of the last successful submission.

use std::{
    future::Future,
    task::Poll,
};

mod auctioneer;
mod bid;
mod block;
mod build_info;
pub mod config;
pub(crate) mod metrics;
mod rollup_channel;
mod sequencer_channel;
mod sequencer_key;
mod streaming_utils;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
pub use build_info::BUILD_INFO;
pub use config::Config;
pub use metrics::Metrics;
use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

/// The [`Auctioneer`] service returned by [`Auctioneer::spawn`].
pub struct Auctioneer {
    shutdown_token: CancellationToken,
    task: Option<JoinHandle<eyre::Result<()>>>,
}

impl Auctioneer {
    /// Spawns the [`Auctioneer`] service.
    ///
    /// # Errors
    /// Returns an error if the Auctioneer cannot be initialized.
    pub fn spawn(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
        let inner = auctioneer::Auctioneer::new(cfg, metrics, shutdown_token.child_token())?;
        let task = tokio::spawn(inner.run());

        Ok(Self {
            shutdown_token,
            task: Some(task),
        })
    }

    /// Shuts down Auctioneer, in turn waiting for its components to shut down.
    ///
    /// # Errors
    /// Returns an error if an error occured during shutdown.
    ///
    /// # Panics
    /// Panics if called twice.
    #[instrument(skip_all, err)]
    pub async fn shutdown(&mut self) -> eyre::Result<()> {
        self.shutdown_token.cancel();
        flatten_join_result(
            self.task
                .take()
                .expect("shutdown must not be called twice")
                .await,
        )
    }
}

impl Future for Auctioneer {
    type Output = eyre::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        use futures::future::FutureExt as _;

        let task = self
            .task
            .as_mut()
            .expect("auctioneer must not be polled after shutdown");
        task.poll_unpin(cx).map(flatten_join_result)
    }
}

fn flatten_join_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
