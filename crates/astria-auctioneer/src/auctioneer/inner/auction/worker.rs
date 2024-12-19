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

use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::primitive::v1::{
    Address,
    RollupId,
    asset,
};
use astria_eyre::eyre::{
    self,
    Context,
    bail,
};
use futures::FutureExt as _;
use sequencer_client::SequencerClientExt;
use tokio::{
    select,
    sync::oneshot,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    Level,
    error,
    info,
    instrument,
};

use super::{
    Summary,
    allocation_rule::FirstPrice,
};
use crate::{
    bundle::Bundle,
    sequencer_channel::SequencerChannel,
    sequencer_key::SequencerKey,
};

pub(super) struct Worker {
    /// The sequencer's ABCI client, used for submitting transactions
    pub(super) sequencer_abci_client: sequencer_client::HttpClient,
    pub(super) sequencer_channel: SequencerChannel,
    pub(super) start_bids: Option<oneshot::Receiver<()>>,
    pub(super) start_timer: Option<oneshot::Receiver<()>>,
    /// Channel for receiving new bundles
    pub(super) bundles: tokio::sync::mpsc::UnboundedReceiver<Arc<Bundle>>,
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
    pub(super) async fn run(mut self) -> eyre::Result<Summary> {
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
        } = auction_result.wrap_err("auction failed while waiting for bids")?;
        let Some(winner) = winner else {
            return Ok(Summary::NoBids);
        };

        let nonce_fetch = nonce_fetch.expect(
            "if the auction loop produced a winner, then a nonce fetch must have been spawned",
        );

        let pending_nonce = match nonce_fetch.now_or_never() {
            Some(Ok(nonce)) => nonce,
            Some(Err(error)) => return Err(error).wrap_err("task to fetch nonce has panicked"),
            None => bail!(
                "task to fetch nonce did not return in time once auction winner was selected and \
                 ready to be submitted"
            ),
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

        let submission_fut = {
            let client = self.sequencer_abci_client.clone();
            tokio::time::timeout(Duration::from_secs(30), async move {
                client
                    .submit_transaction_sync(transaction)
                    .await
                    .wrap_err("submission request failed")
            })
        };
        tokio::pin!(submission_fut);

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
                        .wrap_err("submission of auction winner timed out before receiving a response from Sequencer")
                    {
                        Ok(Ok(rsp)) => Ok(Summary::Submitted(rsp)),
                        Err(err) | Ok(Err(err)) => Err(err),
                    }
                }
            );
        }
    }

    async fn run_auction_loop(&mut self) -> eyre::Result<AuctionItems> {
        let mut latency_margin_timer = None;
        // TODO: do we want to make this configurable to allow for more complex allocation rules?
        let mut allocation_rule = FirstPrice::new();
        let mut auction_is_open = false;

        let mut nonce_fetch = None;
        loop {
            select! {
                biased;

                _ = async { latency_margin_timer.as_mut().unwrap() },
                    if latency_margin_timer.is_some() =>
                {
                    info!("timer is up; bids left unprocessed: {}", self.bundles.len());
                    break Ok(AuctionItems {
                        winner: allocation_rule.winner(),
                        nonce_fetch,
                    })
                }

                Ok(()) = async { self.start_bids.as_mut().unwrap().await },
                    if self.start_bids.is_some() =>
                {
                    let mut channel = self
                        .start_bids
                        .take()
                        .expect("inside an arm that that checks start_bids == Some");
                    channel.close();
                    // TODO: if the timer is already running, report how much time is left for the bids
                    auction_is_open = true;
                }

                Ok(()) = async { self.start_timer.as_mut().unwrap().await },
                    if self.start_timer.is_some() =>
                {
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

                    // TODO: Emit an event to report start and endpoint of the auction.
                    latency_margin_timer = Some(tokio::time::sleep(self.latency_margin));
                    nonce_fetch = Some(tokio::spawn(get_pending_nonce(
                        self.sequencer_channel.clone(),
                        *self.sequencer_key.address(),
                    )));
                }

                // TODO: this is an unbounded channel. Can we process multiple bids at a time?
                Some(bundle) = self.bundles.recv(), if auction_is_open => {
                    allocation_rule.bid(&bundle);
                }

                else => {
                    bail!("all channels are closed; the auction cannot continue")
                }
            }
        }
    }
}

struct AuctionItems {
    winner: Option<Arc<Bundle>>,
    nonce_fetch: Option<JoinHandle<u32>>,
}

/// Fetches the pending nonce for `address` with aggressive retries.
///
/// On failure this method will attempt to immediately refetch the nonce in an
/// infinite loop. It is expected that this future is run in a tokio task, relatively
/// short lived (no longer than the margin timer of the auction), and killed/aborted
/// if not ready by the time an auction result is expected to be available.
#[instrument(skip_all, fields(%address), ret)]
async fn get_pending_nonce(sequencer_channel: SequencerChannel, address: Address) -> u32 {
    loop {
        match sequencer_channel.get_pending_nonce(address).await {
            Ok(nonce) => return nonce,
            Err(error) => {
                error!(%error, "fetching nonce failed; immediately scheduling next fetch")
            }
        }
    }
}
