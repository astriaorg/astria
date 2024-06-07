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
    ensure,
    eyre,
    Context,
};
pub(crate) use builder::Builder;
use sequencer_client::{
    tendermint_rpc,
    tendermint_rpc::endpoint::broadcast::tx_commit,
    Address,
    SequencerClientExt as _,
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
    state,
};

mod builder;
mod signer;
#[cfg(test)]
mod tests;

pub(super) struct Submitter {
    shutdown_token: CancellationToken,
    state: Arc<State>,
    batches_rx: mpsc::Receiver<Batch>,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    signer: SequencerKey,
    sequencer_chain_id: String,
}

impl Submitter {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let actual_chain_id =
            get_sequencer_chain_id(self.sequencer_cometbft_client.clone()).await?;
        ensure!(
            self.sequencer_chain_id == actual_chain_id.to_string(),
            "sequencer_chain_id provided in config does not match chain_id returned from sequencer"
        );
        self.state.set_submitter_ready();

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
                    if let Err(e) = self.process_batch(actions, rollup_height).await {
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

    async fn process_batch(
        &mut self,
        actions: Vec<Action>,
        rollup_height: u64,
    ) -> eyre::Result<()> {
        // get nonce and make unsigned transaction
        let nonce = get_latest_nonce(
            self.sequencer_cometbft_client.clone(),
            self.signer.address,
            self.state.clone(),
        )
        .await?;
        debug!(nonce, "fetched latest nonce");

        let unsigned = UnsignedTransaction {
            actions,
            params: TransactionParams::builder()
                .nonce(nonce)
                .chain_id(&self.sequencer_chain_id)
                .try_build()
                .context(
                    "failed to construct transcation parameters from latest nonce and configured \
                     sequencer chain ID",
                )?,
        };

        // sign transaction
        let signed = unsigned.into_signed(&self.signer.signing_key);
        debug!(tx_hash = %telemetry::display::hex(&signed.sha256_of_proto_encoding()), "signed transaction");

        // submit transaction and handle response
        let rsp = submit_tx(
            self.sequencer_cometbft_client.clone(),
            signed,
            self.state.clone(),
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
            self.state.set_last_rollup_height_submitted(rollup_height);
            self.state.set_last_sequencer_height(rsp.height.value());
            self.state.set_last_sequencer_tx_hash(rsp.hash);
            Ok(())
        }
    }
}

/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(skip_all, fields(%address))]
async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
    state: Arc<State>,
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

    metrics::histogram!(crate::metrics_init::NONCE_FETCH_LATENCY).record(start.elapsed());

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
) -> eyre::Result<tx_commit::Response> {
    let nonce = tx.nonce();
    metrics::gauge!(crate::metrics_init::CURRENT_NONCE).set(nonce);
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

    metrics::histogram!(crate::metrics_init::SEQUENCER_SUBMISSION_LATENCY).record(start.elapsed());

    res
}

async fn get_sequencer_chain_id(
    client: sequencer_client::HttpClient,
) -> eyre::Result<tendermint::chain::Id> {
    use sequencer_client::Client as _;

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch sequencer genesis info; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let genesis: tendermint::Genesis = tryhard::retry_fn(|| client.genesis())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get genesis info from Sequencer after a lot of attempts")?;

    Ok(genesis.chain_id)
}
