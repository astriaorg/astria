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
    eyre,
    Context,
};
pub(crate) use builder::Builder;
pub(super) use builder::Handle;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_commit,
    Address,
    SequencerClientExt,
    SignedTransaction,
};
use signer::SequencerKey;
use state::State;
use tokio::{
    select,
    sync::mpsc,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
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

use super::{
    batch::Batch,
    startup,
    state,
};
use crate::metrics::Metrics;

mod builder;
pub(crate) mod signer;
#[cfg(test)]
mod tests;

pub(super) struct Submitter {
    shutdown_token: CancellationToken,
    startup_handle: startup::InfoHandle,
    state: Arc<State>,
    batches_rx: mpsc::Receiver<Batch>,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    signer: SequencerKey,
    metrics: &'static Metrics,
}

impl Submitter {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let sequencer_chain_id = select! {
            () = self.shutdown_token.cancelled() => {
                info!("submitter received shutdown signal while waiting for startup");
                return Ok(());
            }

            startup_info = self.startup_handle.get_info() => {
                let startup::Info { chain_id, .. } = startup_info.wrap_err("submitter failed to get startup info")?;
                self.state.set_submitter_ready();
                chain_id
            }
        };

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("received shutdown signal");
                    break Ok("shutdown requested");
                }

                batch = self.batches_rx.recv() => {
                    let Some(Batch { actions, rollup_height }) = batch else {
                        info!("received None from batch channel, shutting down");
                        break Err(eyre!("batch channel closed"));
                    };
                    // if batch submission fails, halt the submitter
                    if let Err(e) = process_batch(
                        self.sequencer_cometbft_client.clone(),
                        &self.signer,
                        self.state.clone(),
                        &sequencer_chain_id,
                        actions,
                        rollup_height,
                        self.metrics,
                    ).await {
                        break Err(e);
                    }
                }
            );
        };

        // update status
        self.state.set_sequencer_connected(false);

        // close the channel to signal to batcher that the submitter is shutting down
        self.batches_rx.close();

        match reason {
            Ok(reason) => info!(reason, "submitter shutting down"),
            Err(reason) => {
                error!(%reason, "submitter shutting down");
            }
        }

        Ok(())
    }
}

async fn process_batch(
    sequencer_cometbft_client: sequencer_client::HttpClient,
    sequencer_key: &SequencerKey,
    state: Arc<State>,
    sequencer_chain_id: &str,
    actions: Vec<Action>,
    rollup_height: u64,
    metrics: &'static Metrics,
) -> eyre::Result<()> {
    // get nonce and make unsigned transaction
    let nonce = get_latest_nonce(
        sequencer_cometbft_client.clone(),
        *sequencer_key.address(),
        state.clone(),
        metrics,
    )
    .await
    .wrap_err("failed to get nonce from sequencer")?;
    debug!(nonce, "fetched latest nonce");

    let unsigned = UnsignedTransaction {
        actions,
        params: TransactionParams::builder()
            .nonce(nonce)
            .chain_id(sequencer_chain_id)
            .build(),
    };

    // sign transaction
    let signed = unsigned.into_signed(sequencer_key.signing_key());
    debug!(tx_hash = %telemetry::display::hex(&signed.sha256_of_proto_encoding()), "signed transaction");

    // submit transaction and handle response
    let rsp = submit_tx(
        sequencer_cometbft_client.clone(),
        signed,
        state.clone(),
        metrics,
    )
    .await
    .context("failed to submit transaction to to cometbft")?;
    if let tendermint::abci::Code::Err(check_tx_code) = rsp.check_tx.code {
        error!(
            abci.code = check_tx_code,
            abci.log = rsp.check_tx.log,
            rollup.height = rollup_height,
            "transaction failed to be included in the mempool, aborting."
        );
        Err(eyre!(
            "check_tx failure upon submitting transaction to sequencer"
        ))
    } else if let tendermint::abci::Code::Err(deliver_tx_code) = rsp.tx_result.code {
        error!(
            abci.code = deliver_tx_code,
            abci.log = rsp.tx_result.log,
            rollup.height = rollup_height,
            "transaction failed to be executed in a block, aborting."
        );
        Err(eyre!(
            "deliver_tx failure upon submitting transaction to sequencer"
        ))
    } else {
        // update state after successful submission
        info!(
            sequencer.block = rsp.height.value(),
            sequencer.tx_hash = %rsp.hash,
            rollup.height = rollup_height,
            "withdraw batch successfully executed."
        );
        state.set_last_rollup_height_submitted(rollup_height);
        state.set_last_sequencer_height(rsp.height.value());
        state.set_last_sequencer_tx_hash(rsp.hash);
        Ok(())
    }
}

async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
    state: Arc<State>,
    metrics: &'static Metrics,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
    metrics.increment_nonce_fetch_count();
    let span = Span::current();
    let start = Instant::now();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                metrics.increment_nonce_fetch_failure_count();

                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

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

    state.set_sequencer_connected(res.is_ok());

    metrics.record_nonce_fetch_latency(start.elapsed());

    res
}

/// Submits a `SignedTransaction` to the sequencer with an exponential backoff
#[instrument(
    name = "submit_tx",
    skip_all,
    fields(
        nonce = tx.nonce(),
        transaction.hash = %telemetry::display::hex(&tx.sha256_of_proto_encoding()),
    )
)]
async fn submit_tx(
    client: sequencer_client::HttpClient,
    tx: SignedTransaction,
    state: Arc<State>,
    metrics: &'static Metrics,
) -> eyre::Result<tx_commit::Response> {
    let nonce = tx.nonce();
    metrics.set_current_nonce(nonce);
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
                metrics.increment_sequencer_submission_failure_count();

                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

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

    state.set_sequencer_connected(res.is_ok());

    metrics.record_sequencer_submission_latency(start.elapsed());

    res
}
