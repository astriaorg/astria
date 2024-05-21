use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::protocol::transaction::v1alpha1::{
    Action,
    TransactionParams,
    UnsignedTransaction,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use prost::Message as _;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_commit,
    Address,
    SequencerClientExt as _,
    SignedTransaction,
};
use tendermint::crypto::Sha256;
use tokio::{
    select,
    sync::mpsc,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

mod builder;
mod event;
mod signer;
mod state;

pub(crate) use builder::Builder;
use signer::SequencerSigner;
use state::State;
pub(super) use state::StateSnapshot;
use tracing::{
    debug,
    error,
    info,
    info_span,
    instrument,
    warn,
    Instrument as _,
    Span,
};

use self::event::Event;

pub(super) struct Handle {
    batches_tx: mpsc::Sender<Vec<Event>>,
}

pub(super) struct Executor {
    shutdown_token: CancellationToken,
    state: Arc<State>,
    batches_rx: mpsc::Receiver<Vec<Event>>,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    signer: SequencerSigner,
    sequencer_chain_id: String,
}

impl Executor {
    pub(super) fn subscribe_to_state(&self) -> tokio::sync::watch::Receiver<StateSnapshot> {
        self.state.subscribe()
    }

    pub(super) async fn run(mut self) -> eyre::Result<()> {
        self.state.set_ready();

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("received shutdown signal");
                    break Ok("shutdown requested");
                }

                batch = self.batches_rx.recv() => {
                    let batch = match batch {
                        Some(batch) => batch,
                        None => {
                            info!("received None from batch channel, shutting down");
                            break Err("channel closed");
                        }
                    };

                    let actions = batch.into_iter().map(Action::from).collect::<Vec<Action>>();

                    // get nonce
                    let nonce = get_latest_nonce(self.sequencer_cometbft_client.clone(), self.signer.address).await?;

                    let unsigned = UnsignedTransaction {
                        actions,
                        params: TransactionParams {
                            nonce,
                            chain_id: self.sequencer_chain_id.clone(),
                        },
                    };

                    // sign
                    let signed = unsigned.into_signed(&self.signer.signing_key);

                    // broadcast commit
                }
            )
        };
        // update status

        self.batches_rx.close();

        match reason {
            Ok(reason) => info!(reason, "starting shutdown process"),
            Err(reason) => {
                error!(%reason, "starting shutdown process")
            }
        }

        // handle shutdown

        Ok(())
    }
}

/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(name = "get latest nonce", skip_all, fields(%address))]
async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
    metrics::counter!(crate::metrics_init::NONCE_FETCH_COUNT).increment(1);
    let span = Span::current();
    let start = Instant::now();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                metrics::counter!(crate::metrics_init::NONCE_FETCH_FAILURE_COUNT).increment(1);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: span.clone(),
                    error = err as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed getting latest nonce from sequencer; retrying after backoff",
                );
                async move {}
            },
        );
    let res = tryhard::retry_fn(|| {
        let client = client.clone();
        let span = info_span!(parent: span.clone(), "attempt get nonce");
        async move { client.get_latest_nonce(address).await.map(|rsp| rsp.nonce) }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting latest nonce from sequencer after 1024 attempts");

    metrics::histogram!(crate::metrics_init::NONCE_FETCH_LATENCY).record(start.elapsed());

    res
}

/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(
    name = "submit signed transaction",
    skip_all,
    fields(
        nonce = tx.unsigned_transaction().params.nonce,
        transaction.hash = hex::encode(sha256(&tx.to_raw().encode_to_vec())),
    )
)]
async fn submit_tx(
    client: sequencer_client::HttpClient,
    tx: SignedTransaction,
) -> eyre::Result<tx_commit::Response> {
    let nonce = tx.unsigned_transaction().params.nonce;
    metrics::gauge!(crate::metrics_init::CURRENT_NONCE).set(nonce);

    // TODO: change to info and log tx hash (to match info log in `SubmitFut`'s response handling
    // logic)
    let start = std::time::Instant::now();
    debug!("submitting signed transaction to sequencer");
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                metrics::counter!(crate::metrics_init::SEQUENCER_SUBMISSION_FAILURE_COUNT)
                    .increment(1);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: span.clone(),
                    attempt,
                    wait_duration,
                    error = err as &dyn std::error::Error,
                    "failed sending transaction to sequencer; retrying after backoff",
                );
                async move {}
            },
        );
    let res = tryhard::retry_fn(|| {
        let client = client.clone();
        let tx = tx.clone();
        let span = info_span!(parent: span.clone(), "attempt send");
        async move { client.submit_transaction_commit(tx).await }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed sending transaction after 1024 attempts");

    metrics::histogram!(crate::metrics_init::SEQUENCER_SUBMISSION_LATENCY).record(start.elapsed());

    res
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Sha256;
    Sha256::digest(data)
}
