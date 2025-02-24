//! The event loop that runs an auction.
use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    time::Duration,
};

use astria_core::{
    primitive::v1::{
        asset,
        Address,
        RollupId,
    },
    protocol::transaction::v1::Transaction,
};
use futures::FutureExt as _;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_sync,
    SequencerClientExt as _,
};
use tokio::{
    select,
    sync::oneshot,
    task::{
        JoinError,
        JoinHandle,
    },
    time::{
        sleep,
        Sleep,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
    Instrument as _,
    Level,
};

use super::{
    allocation_rule::FirstPrice,
    Summary,
};
use crate::{
    bid::Bid,
    sequencer_channel::SequencerChannel,
    sequencer_key::SequencerKey,
};

const SUBMISSION_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct Worker {
    /// The sequencer's ABCI client, used for submitting transactions
    pub(super) sequencer_abci_client: sequencer_client::HttpClient,
    pub(super) sequencer_channel: SequencerChannel,
    pub(super) start_bids: Option<oneshot::Receiver<()>>,
    pub(super) start_timer: Option<oneshot::Receiver<()>>,
    /// Channel for receiving new bids.
    pub(super) bids: tokio::sync::mpsc::UnboundedReceiver<Arc<Bid>>,
    /// The time between receiving a block commitment
    pub(super) latency_margin: Duration,
    /// The ID of the auction
    pub(super) id: super::Id,
    /// The key used to sign transactions on the sequencer
    pub(super) sequencer_key: SequencerKey,
    /// Fee asset for submitting transactions
    pub(super) fee_asset_denomination: asset::Denom,
    /// The chain ID used for sequencer transactions
    pub(super) sequencer_chain_id: String,
    /// Rollup ID to submit the auction result to
    pub(super) rollup_id: RollupId,
    pub(super) cancellation_token: CancellationToken,
    /// `last_successful_nonce + 1` is used for submitting an auction winner to
    /// Sequencer if the worker was not able to receive the last pending nonce
    /// from Sequencer in time (in time = by the time the winner was ready to be
    /// submitted). Is usually only unset if no auction was yet submitted (for example
    /// at the beginning of the program).
    pub(super) last_successful_nonce: Option<u32>,
    pub(super) metrics: &'static crate::Metrics,
}

impl Worker {
    // FIXME: consider using Valuable for the return case.
    // See this discussion: https://github.com/tokio-rs/tracing/discussions/1906
    #[instrument(
        skip_all,
        fields(id = %self.id),
        err(level = Level::WARN, Display),
        ret(Display),
    )]
    pub(super) async fn run(mut self) -> Result<Summary, Error> {
        let Some(auction_result) = self
            .cancellation_token
            .clone()
            .run_until_cancelled(self.run_auction_loop())
            .await
        else {
            return Ok(Summary::CancelledDuringAuction);
        };
        let AuctionItems {
            winner,
            nonce_fetch,
        } = auction_result?;
        let Some(winner) = winner else {
            return Ok(Summary::NoBids);
        };

        let nonce_fetch = nonce_fetch.expect(
            "if the auction loop produced a winner, then a nonce fetch must have been spawned",
        );

        let pending_nonce = match nonce_fetch.now_or_never() {
            Some(Ok(nonce)) => nonce,
            Some(Err(source)) => {
                return Err(Error::NonceFetchPanicked {
                    source,
                });
            }
            None if self.last_successful_nonce.is_some() => {
                let nonce = self
                    .last_successful_nonce
                    .expect("in arm that checks for last_successful_nonce == Some")
                    .saturating_add(1);
                info!(
                    "request for latest pending nonce did not return in time; using `{nonce}`
                    instead (last successful nonce + 1)"
                );
                nonce
            }
            None => return Err(Error::NoNonce),
        };

        // TODO: report the pending nonce that we ended up using.
        let transaction = Arc::unwrap_or_clone(winner)
            .into_transaction_body(
                pending_nonce,
                self.rollup_id,
                &self.sequencer_key,
                self.fee_asset_denomination.clone(),
                self.sequencer_chain_id,
            )
            .sign(self.sequencer_key.signing_key());

        // NOTE: Submit fire-and-forget style. If the submission didn't make it in time,
        // it's likey lost.
        // TODO: We can consider providing a very tight retry mechanism. Maybe resubmit once
        // if the response didn't take too long? But it's probably a bad idea to even try.
        // Can we detect if a submission failed due to a bad nonce? In this case, we could
        // immediately ("optimistically") submit with the most recent pending nonce (if the
        // publisher updated it in the meantime) or just nonce + 1 (if it didn't yet update)?

        let submission_fut =
            submit_winner_with_timeout(self.sequencer_abci_client.clone(), transaction);
        tokio::pin!(submission_fut);

        let submission_start = std::time::Instant::now();
        loop {
            select!(
                () = self.cancellation_token.clone().cancelled_owned(),
                    if !self.cancellation_token.is_cancelled() =>
                {
                    info!(
                        "received cancellation token while waiting for Sequencer to respond to \
                         transaction submission; still waiting for submission until timeout"
                     );
                }

                res = &mut submission_fut => {
                    break match res
                    {
                        Ok(response) => {
                            self.metrics.record_auction_winner_submission_success_latency(submission_start.elapsed());
                            Ok(Summary::Submitted { nonce_used: pending_nonce, response, })
                        }
                        Err(err) => {
                            self.metrics.record_auction_winner_submission_error_latency(submission_start.elapsed());
                            Err(err)
                        }
                    }
                }
            );
        }
    }

    async fn run_auction_loop(&mut self) -> Result<AuctionItems, Error> {
        let mut latency_margin_timer = pin!(None::<Sleep>);
        // TODO: do we want to make this configurable to allow for more complex allocation rules?
        let mut allocation_rule = FirstPrice::new();
        let mut auction_is_open = false;

        let mut nonce_fetch = None;

        #[expect(
            clippy::semicolon_if_nothing_returned,
            reason = "we want to pattern match on the latency timer's return value"
        )]
        loop {
            select! {
                biased;

                () = async {
                    Option::as_pin_mut(latency_margin_timer.as_mut())
                        .unwrap()
                        .await
                }, if latency_margin_timer.is_some() => {
                    info!("timer is up; bids left unprocessed: {}", self.bids.len());

                    self.metrics.record_bids_per_auction_dropped_histogram(self.bids.len());
                    self.metrics.record_bids_per_auction_processed_histogram(allocation_rule.bids_seen());

                    let winner = allocation_rule.take_winner();
                    if let Some(winner) = &winner {
                        self.metrics.record_auction_winning_bid_histogram(winner.bid());
                    }

                    break Ok(AuctionItems {
                        winner,
                        nonce_fetch,
                    })
                }

                Ok(()) = async {
                    self.start_bids.as_mut().unwrap().await
                }, if self.start_bids.is_some() => {
                    let mut channel = self
                        .start_bids
                        .take()
                        .expect("inside an arm that that checks start_bids == Some");
                    channel.close();
                    // TODO: if the timer is already running, report how much time is left for the bids
                    auction_is_open = true;
                }

                Ok(()) = async {
                    self.start_timer.as_mut().unwrap().await
                }, if self.start_timer.is_some() => {
                    let mut channel = self
                        .start_timer
                        .take()
                        .expect("inside an arm that checks start_timer == Some");
                    channel.close();
                    if !auction_is_open {
                        info!(
                            "received signal to start the auction timer before signal to start \
                            processing bids; that's ok but eats into the time allotment of the \
                            auction"
                        );
                    }

                    latency_margin_timer.set(Some(sleep(self.latency_margin)));
                    nonce_fetch = Some(spawn_aborting(get_pending_nonce(
                        self.sequencer_channel.clone(),
                        *self.sequencer_key.address(),
                    ).in_current_span()));
                    info!(
                        duration = %humantime::format_duration(self.latency_margin),
                        "started auction timer and request for latest nonce",
                    );
                }

                // TODO: this is an unbounded channel. Can we process multiple bids at a time?
                Some(bid) = self.bids.recv(), if auction_is_open => {
                    allocation_rule.bid(&bid);
                }

                else => {
                    break Err(Error::ChannelsClosed);
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::auctioneer) enum Error {
    #[error("all channels to the auction worker are closed; the auction cannot continue")]
    ChannelsClosed,
    // TODO: Is there a way to identify the winning bid? Do we need it?
    #[error(
        "selected winning bid, but latest nonce was not yet initialized (should only be the case \
         at start of service) and Sequencer did not return the latest nonce in time"
    )]
    NoNonce,
    #[error("task fetching nonce from Sequencer panicked")]
    NonceFetchPanicked { source: tokio::task::JoinError },
    #[error(
        "submission of winner to Sequencer elapsed after {}",
        humantime::format_duration(SUBMISSION_TIMEOUT)
    )]
    SubmissionElapsed { source: tokio::time::error::Elapsed },
    #[error("encountered an error when sending the winning bid to Sequencer")]
    SubmissionFailed {
        source: sequencer_client::extension_trait::Error,
    },
}

fn spawn_aborting<F>(fut: F) -> AbortJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    AbortJoinHandle(tokio::spawn(fut))
}

struct AuctionItems {
    winner: Option<Arc<Bid>>,
    nonce_fetch: Option<AbortJoinHandle<u32>>,
}

/// A wrapper around [`JoinHandle`] that aborts the task rather than disassocating.
#[derive(Debug)]
pub(crate) struct AbortJoinHandle<T>(JoinHandle<T>);

impl<T> Drop for AbortJoinHandle<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> From<JoinHandle<T>> for AbortJoinHandle<T> {
    fn from(handle: JoinHandle<T>) -> Self {
        AbortJoinHandle(handle)
    }
}

impl<T> Future for AbortJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}

/// Fetches the pending nonce for `address` with aggressive retries.
///
/// On failure this method will attempt to immediately refetch the nonce in an
/// infinite loop. It is expected that this future is run in a tokio task, relatively
/// short lived (no longer than the margin timer of the auction), and killed/aborted
/// if not ready by the time an auction result is expected to be available.
#[instrument(skip_all, fields(%address), ret(Display))]
async fn get_pending_nonce(sequencer_channel: SequencerChannel, address: Address) -> u32 {
    loop {
        match sequencer_channel.get_pending_nonce(address).await {
            Ok(nonce) => return nonce,
            Err(error) => {
                error!(%error, "fetching nonce failed; immediately scheduling next fetch");
            }
        }
    }
}

async fn submit_winner_with_timeout(
    client: sequencer_client::HttpClient,
    transaction: Transaction,
) -> Result<tx_sync::Response, Error> {
    // TODO(janis): starting from v0.35.0, tendermint-rpc provides a
    // mechanism to timeout requests on its http client, so that this
    // explicit timeout can be removed.
    match tokio::time::timeout(
        SUBMISSION_TIMEOUT,
        client.submit_transaction_sync(transaction),
    )
    .await
    {
        Ok(Ok(rsp)) => Ok(rsp),
        Ok(Err(source)) => Err(Error::SubmissionFailed {
            source,
        }),
        Err(source) => Err(Error::SubmissionElapsed {
            source,
        }),
    }
}
