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
    ContextCompat,
    OptionExt as _,
};
pub(crate) use builder::Builder;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_sync,
    Address,
    SequencerClientExt,
};
use telemetry::display::base64;
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
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

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.0
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
    start_processing_bids_tx: Option<oneshot::Sender<()>>,
    start_timer_tx: Option<oneshot::Sender<()>>,
    abort_tx: Option<oneshot::Sender<()>>,
    new_bundles_tx: mpsc::Sender<Bundle>,
}

impl Handle {
    pub(crate) fn abort(&mut self) -> eyre::Result<()> {
        let _ = self
            .abort_tx
            .take()
            .ok_or_eyre("should only send reorg signal to a given auction once")?;

        Ok(())
    }

    pub(crate) fn start_processing_bids(&mut self) -> eyre::Result<()> {
        let _ = self
            .start_processing_bids_tx
            .take()
            .ok_or_eyre("should only send executed signal to a given auction once")?
            .send(());
        Ok(())
    }

    pub(crate) fn start_timer(&mut self) -> eyre::Result<()> {
        let _ = self
            .start_timer_tx
            .take()
            .ok_or_eyre("should only send block commitment signal to a given auction once")?
            .send(());

        Ok(())
    }

    pub(crate) fn try_send_bundle(&mut self, bundle: Bundle) -> eyre::Result<()> {
        self.new_bundles_tx
            .try_send(bundle)
            .wrap_err("bid channel full")?;

        Ok(())
    }
}

// TODO: should this be the same object as the auction?
pub(crate) struct Auction {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    shutdown_token: CancellationToken,

    /// The sequencer's gRPC client, used for fetching pending nonces
    sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
    /// The sequencer's ABCI client, used for submitting transactions
    sequencer_abci_client: sequencer_client::HttpClient,
    /// Channel for receiving the executed block signal to start processing bundles
    start_processing_bids_rx: oneshot::Receiver<()>,
    /// Channel for receiving the block commitment signal to start the latency margin timer
    start_timer_rx: oneshot::Receiver<()>,
    /// Channel for receiving the reorg signal
    abort_rx: oneshot::Receiver<()>,
    /// Channel for receiving new bundles
    new_bundles_rx: mpsc::Receiver<Bundle>,
    /// The time between receiving a block commitment
    latency_margin: Duration,
    /// The ID of the auction
    auction_id: Id,
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
        let mut allocation_rule = FirstPrice::new();
        let mut auction_is_open = false;

        let mut nonce_fetch: Option<tokio::task::JoinHandle<eyre::Result<u32>>> = None;

        let auction_result = loop {
            select! {
                biased;

                () = self.shutdown_token.cancelled() => break Err(eyre!("received shutdown signal")),

                signal = &mut self.abort_rx => {
                    match signal {
                        Ok(()) => {
                            break Err(eyre!("reorg signal received"))
                        }
                        Err(_) => {
                            return Err(eyre!("reorg signal channel closed"));
                        }
                    }
                    //
                }

                // get the auction winner when the timer expires
                _ = async { latency_margin_timer.as_mut().unwrap() }, if latency_margin_timer.is_some() => {
                    break Ok(allocation_rule.highest_bid());
                }

                signal = &mut self.start_processing_bids_rx, if !auction_is_open => {
                    if let Err(e) = signal {
                        break Err(e).wrap_err("exec signal channel closed");
                    }
                    // set auction to open so it starts collecting bids
                    auction_is_open = true;
                }

                signal = &mut self.start_timer_rx, if auction_is_open => {
                    if let Err(e) = signal {
                        break Err(e).wrap_err("commit signal channel closed");
                    }
                    // set the timer
                    latency_margin_timer = Some(tokio::time::sleep(self.latency_margin));

                    let client = self.sequencer_grpc_client.clone();
                    let address = self.sequencer_key.address().clone();

                    // we wait for commit because we want the pending nonce from after the commit
                    // TODO: fix lifetime issue with passing metrics here?
                    nonce_fetch = Some(tokio::task::spawn(async move {
                        get_pending_nonce(client, address).await
                    }));
                }

                Some(bundle) = self.new_bundles_rx.recv(), if auction_is_open => {
                    if allocation_rule.bid(bundle.clone()) {
                        info!(
                            auction.id = %base64(self.auction_id),
                            bundle.bid = %bundle.bid(),
                            "received new highest bid"
                        );
                    } else {
                        debug!(
                            auction.id = %base64(self.auction_id),
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
                        info!(auction.id = %base64(self.auction_id), auction.result = %resp.log, "auction result submitted to sequencer");
                        Ok(())
                    },
                    Err(e) => {
                        error!(auction.id = %base64(self.auction_id), err = %e, "failed to submit auction result to sequencer");
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
        let address = address.clone();

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
    _metrics: &'static Metrics,
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
