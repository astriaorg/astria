use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    generated::sequencerblock::v1alpha1::{
        sequencer_service_client::{
            self,
            SequencerServiceClient,
        },
        GetPendingNonceRequest,
    },
    protocol::transaction::v1alpha1::{
        Action,
        TransactionParams,
        UnsignedTransaction,
    },
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    Context,
};
pub(crate) use builder::Builder;
pub(super) use builder::Handle;
use sequencer_client::{
    tendermint_rpc::endpoint::{
        broadcast::tx_sync,
        tx,
    },
    Address,
    SequencerClientExt,
    SignedTransaction,
};
use signer::SequencerKey;
use state::State;
use tokio::{
    select,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
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

pub(super) struct Submitter {
    shutdown_token: CancellationToken,
    startup_handle: startup::InfoHandle,
    state: Arc<State>,
    batches_rx: mpsc::Receiver<Batch>,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    sequencer_grpc_endpoint: String,
    signer: SequencerKey,
    metrics: &'static Metrics,
}

impl Submitter {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let (sequencer_chain_id, sequencer_grpc_client) = select! {
            () = self.shutdown_token.cancelled() => {
                info!("submitter received shutdown signal while waiting for startup");
                return Ok(());
            }

            startup_info = self.startup_handle.get_info() => {
                let startup::Info { chain_id, .. } = startup_info.wrap_err("submitter failed to get startup info")?;

                let sequencer_grpc_client = sequencer_service_client::SequencerServiceClient::connect(
                    self.sequencer_grpc_endpoint.clone(),
                ).await.wrap_err("failed to connect to sequencer gRPC endpoint")?;

                self.state.set_submitter_ready();
                (chain_id, sequencer_grpc_client)
            }
        };
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
                        break Err(eyre!("batch channel closed"));
                    };

                    // if batch submission fails, halt the submitter
                    if let Err(e) = self.process_batch(
                        sequencer_grpc_client.clone(),
                        &sequencer_chain_id,
                        actions,
                        rollup_height,
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

    #[instrument(skip_all)]
    async fn process_batch(
        &self,
        sequencer_grpc_client: SequencerServiceClient<Channel>,
        sequencer_chain_id: &String,
        actions: Vec<Action>,
        rollup_height: u64,
    ) -> eyre::Result<()> {
        let Self {
            sequencer_cometbft_client,
            signer,
            state,
            metrics,
            ..
        } = self;
        // get nonce and make unsigned transaction
        let nonce = get_pending_nonce(
            sequencer_grpc_client.clone(),
            *signer.address(),
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
        let signed = unsigned.into_signed(signer.signing_key());
        debug!(tx_hash = %telemetry::display::hex(&signed.sha256_of_proto_encoding()), "signed transaction");

        // submit transaction and handle response
        let (check_tx, tx_response) = submit_tx(
            sequencer_cometbft_client.clone(),
            signed,
            state.clone(),
            metrics,
        )
        .await
        .context("failed to submit transaction to cometbft")?;
        if let tendermint::abci::Code::Err(check_tx_code) = check_tx.code {
            error!(
                abci.code = check_tx_code,
                abci.log = check_tx.log,
                rollup.height = rollup_height,
                "transaction failed to be included in the mempool, aborting."
            );
            Err(eyre!(
                "check_tx failure upon submitting transaction to sequencer"
            ))
        } else if let tendermint::abci::Code::Err(deliver_tx_code) = tx_response.tx_result.code {
            error!(
                abci.code = deliver_tx_code,
                abci.log = tx_response.tx_result.log,
                rollup.height = rollup_height,
                "transaction failed to be executed in a block, aborting."
            );
            Err(eyre!(
                "deliver_tx failure upon submitting transaction to sequencer"
            ))
        } else {
            // update state after successful submission
            info!(
                sequencer.block = tx_response.height.value(),
                sequencer.tx_hash = %tx_response.hash,
                rollup.height = rollup_height,
                "withdraw batch successfully executed."
            );
            state.set_last_rollup_height_submitted(rollup_height);
            state.set_last_sequencer_height(tx_response.height.value());
            state.set_last_sequencer_tx_hash(tx_response.hash);
            Ok(())
        }
    }
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
) -> eyre::Result<(tx_sync::Response, tx::Response)> {
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
    let check_tx = tryhard::retry_fn(|| {
        let client = client.clone();
        let tx = tx.clone();
        let span = info_span!(parent: span.clone(), "attempt send");
        async move { client.submit_transaction_sync(tx).await }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed sending transaction after 1024 attempts");

    state.set_sequencer_connected(check_tx.is_ok());

    metrics.record_sequencer_submission_latency(start.elapsed());

    let check_tx = check_tx?;

    ensure!(check_tx.code.is_ok(), "check_tx failed: {}", check_tx.log);

    let tx_response = client.wait_for_tx_inclusion(check_tx.hash).await?;

    Ok((check_tx, tx_response))
}

pub(crate) async fn get_pending_nonce(
    client: sequencer_service_client::SequencerServiceClient<Channel>,
    address: Address,
    state: Arc<State>,
    metrics: &'static Metrics,
) -> eyre::Result<u32> {
    debug!("fetching pending nonce from sequencing");
    let start = std::time::Instant::now();
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt, next_delay: Option<Duration>, err: &tonic::Status| {
                metrics.increment_nonce_fetch_failure_count();
                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    error = err as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed getting pending nonce from sequencing; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| {
        let mut client = client.clone();
        let span = info_span!(parent: span.clone(), "attempt get pending nonce");
        async move {
            client
                .get_pending_nonce(GetPendingNonceRequest {
                    address: Some(address.into_raw()),
                })
                .await
                .map(|rsp| rsp.into_inner().inner)
        }
        .instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting pending nonce from sequencing after 1024 attempts");

    state.set_sequencer_connected(res.is_ok());
    metrics.increment_nonce_fetch_count();

    metrics.record_nonce_fetch_latency(start.elapsed());

    res
}
