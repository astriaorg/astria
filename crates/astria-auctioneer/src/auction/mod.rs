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
//! The auction may also be aborted at any point before the timer expires by receiving a
//! `Command::Abort`. This will cause the auction to return early without submitting a winner,
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

mod builder;
use std::time::Duration;

use allocation_rule::FirstPrice;
use astria_core::{
    generated::sequencerblock::v1::{
        sequencer_service_client::SequencerServiceClient,
        GetPendingNonceRequest,
    },
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1::Transaction,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
    OptionExt as _,
};
pub(crate) use builder::Builder;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_sync,
    Address,
    SequencerClientExt,
};
use tokio::{
    select,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
    Instrument,
};

use crate::{
    bundle::Bundle,
    sequencer_key::SequencerKey,
    Metrics,
};

pub(crate) mod manager;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(crate) struct Id([u8; 32]);

impl Id {
    pub(crate) fn from_sequencer_block_hash(block_hash: [u8; 32]) -> Self {
        Self(block_hash)
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

pub(crate) use manager::Manager;

mod allocation_rule;

enum Command {
    StartProcessingBids,
    StartTimer,
    Abort,
}

pub(crate) struct Handle {
    commands_tx: mpsc::Sender<Command>,
    new_bundles_tx: mpsc::Sender<Bundle>,
}

impl Handle {
    pub(crate) fn try_abort(&mut self) -> eyre::Result<()> {
        self.commands_tx
            .try_send(Command::Abort)
            .wrap_err("unable to send abort command to auction")?;

        Ok(())
    }

    pub(crate) fn start_processing_bids(&mut self) -> eyre::Result<()> {
        self.commands_tx
            .try_send(Command::StartProcessingBids)
            .wrap_err("unable to send command to start processing bids to auction")?;
        Ok(())
    }

    pub(crate) fn start_timer(&mut self) -> eyre::Result<()> {
        self.commands_tx
            .try_send(Command::StartTimer)
            .wrap_err("unable to send command to start time to auction")?;

        Ok(())
    }

    pub(crate) fn try_send_bundle(&mut self, bundle: Bundle) -> eyre::Result<()> {
        self.new_bundles_tx
            .try_send(bundle)
            .wrap_err("bid channel full")?;

        Ok(())
    }
}

pub(crate) struct Auction {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    shutdown_token: CancellationToken,

    /// The sequencer's gRPC client, used for fetching pending nonces
    sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
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
}

impl Auction {
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let mut latency_margin_timer = None;
        // TODO: do we want to make this configurable to allow for more complex allocation rules?
        let mut allocation_rule = FirstPrice::new();
        let mut auction_is_open = false;

        let mut nonce_fetch: Option<tokio::task::JoinHandle<eyre::Result<u32>>> = None;

        let auction_result = loop {
            select! {
                biased;

                () = self.shutdown_token.cancelled() => break Err(eyre!("received shutdown signal")),

                // get the auction winner when the timer expires
                _ = async { latency_margin_timer.as_mut().unwrap() }, if latency_margin_timer.is_some() => {
                    break Ok(allocation_rule.winner());
                }

                Some(cmd) = self.commands_rx.recv() => {
                    match cmd {
                        Command::Abort => {
                            // abort the auction early
                            break Err(eyre!("auction {} received abort signal", self.id));
                        },
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

                            // we wait for commit because we want the pending nonce from the committed block
                            nonce_fetch = {
                                let client = self.sequencer_grpc_client.clone();
                                let &address = self.sequencer_key.address();
                                Some(tokio::task::spawn(async move { get_pending_nonce(client, address, self.metrics).await }))
                            };
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

        // TODO: separate the rest of this to a different object, e.g. AuctionResult?
        // TODO: flatten this or get rid of the option somehow?
        // await the nonce fetch result
        let nonce = nonce_fetch
            .expect(
                "should have received commit and fetched pending nonce before exiting the auction \
                 loop",
            )
            .await
            .wrap_err("get_pending_nonce task failed")?
            .wrap_err("failed to fetch nonce")?;

        // serialize, sign and submit to the sequencer
        let transaction_body = auction_result
            .wrap_err("auction failed unexpectedly")?
            .ok_or_eyre("auction ended with no winning bid")?
            .into_transaction_body(
                nonce,
                self.rollup_id,
                self.sequencer_key.clone(),
                self.fee_asset_denomination.clone(),
                self.sequencer_chain_id,
            );

        let transaction = transaction_body.sign(self.sequencer_key.signing_key());

        let submission_result = select! {
            biased;

            // TODO: should this be Ok(())? or Ok("received shutdown signal")?
            () = self.shutdown_token.cancelled() => Err(eyre!("received shutdown signal during auction result submission")),

            result = submit_transaction(self.sequencer_abci_client.clone(), transaction, self.metrics) => {
                // TODO: how to handle submission failure better?
                match result {
                    Ok(resp) => {
                        // TODO: handle failed submission instead of just logging the result
                        info!(auction.id = %self.id, auction.result = %resp.log, "auction result submitted to sequencer");
                        Ok(())
                    },
                    Err(e) => {
                        error!(auction.id = %self.id, err = %e, "failed to submit auction result to sequencer");
                        Err(e).wrap_err("failed to submit auction result to sequencer")
                    },
                }
            }
        };
        submission_result
    }
}

#[instrument(skip_all, fields(%address, err))]
async fn get_pending_nonce(
    client: SequencerServiceClient<tonic::transport::Channel>,
    address: Address,
    // TODO: emit metrics here
    #[allow(unused_variables)] metrics: &'static Metrics,
) -> eyre::Result<u32> {
    let span = tracing::Span::current();
    let retry_cfg = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(2))
        .on_retry(
            move |attempt: u32, next_delay: Option<Duration>, error: &tonic::Status| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to get pending nonce failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let nonce = tryhard::retry_fn(|| {
        let mut client = client.clone();

        async move {
            client
                .get_pending_nonce(GetPendingNonceRequest {
                    address: Some(address.into_raw()),
                })
                .await
        }
    })
    .with_config(retry_cfg)
    .in_current_span()
    .await
    .wrap_err("failed to get pending nonce")?
    .into_inner()
    .inner;

    Ok(nonce)
}

async fn submit_transaction(
    client: sequencer_client::HttpClient,
    transaction: Transaction,
    // TODO: emit metrics here
    #[allow(unused_variables)] metrics: &'static Metrics,
) -> eyre::Result<tx_sync::Response> {
    let span = tracing::Span::current();
    let retry_cfg = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(2))
        .on_retry(
            move |attempt: u32,
                  next_delay: Option<Duration>,
                  error: &sequencer_client::extension_trait::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to submit transaction failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    tryhard::retry_fn(|| {
        let client = client.clone();
        let transaction = transaction.clone();

        async move { client.submit_transaction_sync(transaction).await }
    })
    .with_config(retry_cfg)
    .in_current_span()
    .await
    .wrap_err("failed to submit transaction")
}
