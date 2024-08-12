use std::str::FromStr;
/// ! The `Executor` is responsible for:
/// - Nonce management
/// - Transaction signing
/// - Managing the connection to the sequencer
/// - Submitting transactions to the sequencer
use std::{
    collections::VecDeque,
    pin::Pin,
    task::Poll,
    time::Duration,
};
use ethers::abi::AbiEncode;

use astria_core::{
    crypto::SigningKey,
    generated::{
        composer::v1alpha1::{
            BuilderBundle,
            BuilderBundlePacket,
        },
        sequencerblock::v1alpha1::RollupData,
    },
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::{
            action::SequenceAction,
            SignedTransaction,
            TransactionParams,
            UnsignedTransaction,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use ethers::signers::Signer;
use ethers::utils::hash_message;
use futures::{
    future::{
        self,
        Fuse,
        FusedFuture as _,
        FutureExt as _,
    },
    ready,
    Future,
};
use pin_project_lite::pin_project;
use prost::Message as _;
use sequencer_client::{
    tendermint_rpc::{
        endpoint::broadcast::tx_sync,
        Client as _,
    },
    Address,
    SequencerClientExt as _,
};
use tendermint::crypto::Sha256;
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            error::SendTimeoutError,
        },
        watch,
    },
    time::{
        self,
        Instant,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    info_span,
    instrument,
    instrument::Instrumented,
    warn,
    Instrument,
    Span,
};

use self::bundle_factory::SizedBundle;
use crate::{
    executor::bundle_factory::{
        BundleFactory,
        SizedBundleReport,
    },
    metrics::Metrics,
};

mod bundle_factory;

pub(crate) mod builder;
mod client;
mod simulator;
#[cfg(test)]
mod tests;

pub(crate) use builder::Builder;

use crate::executor::simulator::BundleSimulator;

// Duration to wait for the executor to drain all the remaining bundles before shutting down.
// This is 16s because the timeout for the higher level executor task is 17s to shut down.
// The extra second is to prevent the higher level executor task from timing out before the
// executor has a chance to drain all the remaining bundles.
const BUNDLE_DRAINING_DURATION: Duration = Duration::from_secs(16);

type StdError = dyn std::error::Error;
#[derive(Debug, thiserror::Error)]
pub(crate) enum EnsureChainIdError {
    #[error("failed to obtain sequencer chain ID after multiple retries")]
    GetChainId(#[source] sequencer_client::tendermint_rpc::Error),
    #[error("expected chain ID `{expected}`, but received `{actual}`")]
    WrongChainId {
        expected: String,
        actual: tendermint::chain::Id,
    },
}
/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
pub(super) struct Executor {
    // The status of this executor
    status: watch::Sender<Status>,
    // Channel for receiving `SequenceAction`s to be bundled.
    serialized_rollup_transactions: mpsc::Receiver<SequenceAction>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: sequencer_client::HttpClient,
    // The chain id used for submission of transactions to the sequencer.
    sequencer_chain_id: String,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // The sequencer address associated with the private key
    address: Address,
    // Milliseconds for bundle timer to make sure bundles are submitted at least once per block.
    block_time: tokio::time::Duration,
    // Max bytes in a sequencer action bundle
    max_bytes_per_bundle: usize,
    // Max amount of `SizedBundle`s that can be in the `BundleFactory`'s `finished` queue.
    bundle_queue_capacity: usize,
    // Token to signal the executor to stop upon shutdown.
    shutdown_token: CancellationToken,
    // `BundleSimulator` simulates the execution of a bundle of transactions.
    bundle_simulator: BundleSimulator,
    // The rollup id associated with this executor
    rollup_id: RollupId,
    // The asset used for sequencer fees
    fee_asset: asset::Denom,
    metrics: &'static Metrics,
}

#[derive(Clone)]
pub(super) struct Handle {
    serialized_rollup_transactions_tx: mpsc::Sender<SequenceAction>,
}

impl Handle {
    fn new(serialized_rollup_transactions_tx: mpsc::Sender<SequenceAction>) -> Self {
        Self {
            serialized_rollup_transactions_tx,
        }
    }

    pub(super) async fn send_timeout(
        &self,
        sequence_action: SequenceAction,
        timeout: Duration,
    ) -> Result<(), SendTimeoutError<SequenceAction>> {
        self.serialized_rollup_transactions_tx
            .send_timeout(sequence_action, timeout)
            .await
    }
}

#[derive(Debug)]
pub(super) struct Status {
    is_connected: bool,
}

impl Status {
    pub(super) fn new() -> Self {
        Self {
            is_connected: false,
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        self.is_connected
    }
}

impl Executor {
    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    async fn simulate_and_submit_bundle(
        &self,
        nonce: u32,
        bundle: SizedBundle,
        metrics: &'static Metrics,
    ) -> eyre::Result<Fuse<Instrumented<SubmitFut>>> {
        let bundle_simulator = self.bundle_simulator.clone();

        // simulate the bundle
        let bundle_simulation_result = bundle_simulator
            .simulate_bundle(bundle.clone())
            .await
            .wrap_err("failed to simulate bundle")?;

        let rollup_data_items: Vec<RollupData> = bundle_simulation_result
            .included_actions()
            .iter()
            .map(|action| action.to_raw())
            .collect();

        let builder_bundle = BuilderBundle {
            transactions: rollup_data_items,
            parent_hash: bundle_simulation_result.parent_hash().to_vec(),
        };

        let encoded_builder_bundle = builder_bundle.encode_to_vec();
        let private_key = "0xd7c8dffd7a3898d1be53b5eccd6b1630fa8fe04fd30c5ecf700f1752c3e7e489";
        let wallet = ethers::signers::Wallet::from_str(private_key)
            .wrap_err("failed to parse private key")?;
        let msg_hash = hash_message(encoded_builder_bundle.clone());
        let signature = wallet
            .sign_message(encoded_builder_bundle.clone())
            .await
            .wrap_err("failed to sign builder bundle packet")?;

        // create a top of block bundle
        let mut builder_bundle_packet = BuilderBundlePacket {
            bundle: Some(builder_bundle),
            signature: signature.to_vec(),
            message_hash: msg_hash.encode()
        };
        let encoded_builder_bundle_packet = builder_bundle_packet.encode_to_vec();

        // we can give the BuilderBundlePacket the highest bundle max size possible
        // since this is the only sequence action we are sending
        // TODO - parameterize the max bundle size
        let mut final_bundle = SizedBundle::new(200000);
        if let Err(e) = final_bundle.try_push(SequenceAction {
            rollup_id: self.rollup_id,
            data: encoded_builder_bundle_packet.into(),
            fee_asset: self.fee_asset.clone(),
        }) {
            return Err(eyre::Report::from(e));
        }

        Ok(SubmitFut {
            client: self.sequencer_client.clone(),
            address: self.address,
            nonce,
            chain_id: self.sequencer_chain_id.clone(),
            signing_key: self.sequencer_key.clone(),
            state: SubmitState::NotStarted,
            bundle: final_bundle,
            metrics,
        }
        .in_current_span()
        .fuse())
    }

    /// Create a future to submit a bundle to the sequencer.
    #[instrument(skip_all, fields(nonce.initial = %nonce))]
    fn submit_bundle(
        &self,
        nonce: u32,
        bundle: SizedBundle,
        metrics: &'static Metrics,
    ) -> Fuse<Instrumented<SubmitFut>> {
        SubmitFut {
            client: self.sequencer_client.clone(),
            address: self.address,
            nonce,
            chain_id: self.sequencer_chain_id.clone(),
            signing_key: self.sequencer_key.clone(),
            state: SubmitState::NotStarted,
            bundle,
            metrics,
        }
        .in_current_span()
        .fuse()
    }

    /// Run the Executor loop, calling `process_bundle` on each bundle received from the channel.
    ///
    /// # Errors
    /// An error is returned if connecting to the sequencer fails.
    #[instrument(skip_all, fields(address = %self.address))]
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        select!(
            biased;
            () = self.shutdown_token.cancelled() => {
                info!("received shutdown signal while running initialization routines; exiting");
                return Ok(());
            }

            res = self.pre_run_checks() => {
                res.wrap_err("required pre-run checks failed")?;
            }
        );
        let mut submission_fut: Fuse<Instrumented<SubmitFut>> = Fuse::terminated();
        let mut nonce = get_latest_nonce(self.sequencer_client.clone(), self.address, self.metrics)
            .await
            .wrap_err("failed getting initial nonce from sequencer")?;

        self.metrics.set_current_nonce(nonce);

        let block_timer = time::sleep(self.block_time);
        tokio::pin!(block_timer);
        let mut bundle_factory =
            BundleFactory::new(self.max_bytes_per_bundle, self.bundle_queue_capacity);

        let reset_time = || {
            Instant::now()
                .checked_add(self.block_time)
                .expect("block_time should not be large enough to cause an overflow")
        };

        self.status.send_modify(|status| status.is_connected = true);

        let reason = loop {
            select! {
                biased;

                () = self.shutdown_token.cancelled() => {
                    break Ok("received shutdown signal");
                }
                // process submission result and update nonce
                rsp = &mut submission_fut, if !submission_fut.is_terminated() => {
                    match rsp {
                        Ok(new_nonce) => nonce = new_nonce,
                        Err(error) => {
                            error!(%error, "failed submitting bundle to sequencer; aborting executor");
                            break Err(error).wrap_err("failed submitting bundle to sequencer");
                        }
                    }
                    block_timer.as_mut().reset(reset_time());
                }

                Some(next_bundle) = future::ready(bundle_factory.next_finished()), if submission_fut.is_terminated() => {
                    let bundle = next_bundle.pop();
                    if !bundle.is_empty() {
                        submission_fut = self.simulate_and_submit_bundle(nonce, bundle, self.metrics).await.wrap_err("failed to simulate and submit bundle")?;
                    }
                }

                // receive new seq_action and bundle it. will not pull from the channel if `bundle_factory` is full
                Some(seq_action) = self.serialized_rollup_transactions.recv(), if !bundle_factory.is_full() => {
                    let rollup_id = seq_action.rollup_id;

                    if let Err(e) = bundle_factory.try_push(seq_action) {
                        self.metrics.increment_txs_dropped_too_large(&rollup_id);
                        warn!(
                            rollup_id = %rollup_id,
                            error = &e as &StdError,
                            "failed to bundle transaction, dropping it."
                        );
                    }
                }

                // try to preempt current bundle if the timer has ticked without submitting the next bundle
                () = &mut block_timer, if submission_fut.is_terminated() => {
                    let bundle = bundle_factory.pop_now();
                    if bundle.is_empty() {
                        debug!("block timer ticked, but no bundle to submit to sequencer");
                        block_timer.as_mut().reset(reset_time());
                    } else {
                        debug!(
                            "forcing bundle submission to sequencer due to block timer"
                        );
                        submission_fut = self.simulate_and_submit_bundle(nonce, bundle, self.metrics).await.wrap_err("failed to simulate and submit bundle")?;
                    }
                }
            }
        };

        self.status
            .send_modify(|status| status.is_connected = false);

        // close the channel to avoid receiving any other txs before we drain the remaining
        // sequence actions
        self.serialized_rollup_transactions.close();

        match &reason {
            Ok(reason) => {
                info!(reason, "starting shutdown process");
            }
            Err(reason) => {
                error!(%reason, "executor exited with error");
                // we error out because of a failure to submit a bundle to the sequencer
                // we do not want to proceed with the shutdown process in this case
                return Err(eyre!(reason.to_string()));
            }
        };

        let mut bundles_to_drain: VecDeque<SizedBundle> = VecDeque::new();
        let mut bundles_drained: Option<u64> = Some(0);

        info!("draining already received transactions");

        // drain the receiver channel
        while let Ok(seq_action) = self.serialized_rollup_transactions.try_recv() {
            let rollup_id = seq_action.rollup_id;

            if let Err(e) = bundle_factory.try_push(seq_action) {
                self.metrics.increment_txs_dropped_too_large(&rollup_id);
                warn!(
                    rollup_id = %rollup_id,
                    error = &e as &StdError,
                    "failed to bundle transaction, dropping it."
                );
            }
        }

        // when shutting down, drain all the remaining bundles and submit to the sequencer
        // to avoid any bundle loss.
        loop {
            let bundle = bundle_factory.pop_now();
            if bundle.is_empty() {
                break;
            }

            bundles_to_drain.push_back(bundle);
        }

        info!(
            no_of_bundles_to_drain = bundles_to_drain.len(),
            "submitting remaining transaction bundles to sequencer"
        );

        let shutdown_logic = async {
            // wait for the last bundle to be submitted
            if !submission_fut.is_terminated() {
                info!(
                    "waiting for the last bundle of transactions to be submitted to the sequencer"
                );
                match submission_fut.await {
                    Ok(new_nonce) => {
                        debug!(
                            new_nonce = new_nonce,
                            "successfully submitted bundle of transactions"
                        );

                        nonce = new_nonce;
                    }
                    Err(error) => {
                        error!(%error, "failed submitting bundle to sequencer during shutdown; \
                                aborting shutdown");

                        return Err(error);
                    }
                }
            }

            while let Some(bundle) = bundles_to_drain.pop_front() {
                match self
                    .submit_bundle(nonce, bundle.clone(), self.metrics)
                    .await
                {
                    Ok(new_nonce) => {
                        debug!(
                            bundle = %telemetry::display::json(&SizedBundleReport(&bundle)),
                            new_nonce = new_nonce,
                            "successfully submitted transaction bundle"
                        );

                        nonce = new_nonce;
                        bundles_drained = bundles_drained.and_then(|value| value.checked_add(1));
                    }
                    Err(error) => {
                        error!(
                            bundle =  %telemetry::display::json(&SizedBundleReport(&bundle)),
                            %error,
                            "failed submitting bundle to sequencer during shutdown; \
                                aborting shutdown"
                        );
                        // if we can't submit a bundle after multiple retries, we can abort
                        // the shutdown process

                        return Err(error);
                    }
                }
            }

            Ok(())
        };

        match tokio::time::timeout(BUNDLE_DRAINING_DURATION, shutdown_logic).await {
            Ok(Ok(())) => info!("executor shutdown tasks completed successfully"),
            Ok(Err(error)) => error!(%error, "executor shutdown tasks failed"),
            Err(error) => error!(%error, "executor shutdown tasks failed to complete in time"),
        }

        let number_of_submitted_bundles = if let Some(value) = bundles_drained {
            value.to_string()
        } else {
            format!("more than {}", u64::MAX)
        };
        if bundles_to_drain.is_empty() {
            info!(
                %number_of_submitted_bundles,
                "submitted all outstanding bundles to sequencer during shutdown"
            );
        } else {
            // log all the bundles that have not been drained
            let report: Vec<SizedBundleReport> =
                bundles_to_drain.iter().map(SizedBundleReport).collect();

            warn!(
                %number_of_submitted_bundles,
                number_of_missing_bundles = report.len(),
                missing_bundles = %telemetry::display::json(&report),
                "unable to drain all bundles within the allocated time"
            );
        }

        reason.map(|_| ())
    }

    /// Performs initialization checks prior to running the executor
    async fn pre_run_checks(&self) -> eyre::Result<()> {
        self.ensure_chain_id_is_correct().await?;
        Ok(())
    }

    /// Performs check to ensure the configured chain ID matches the remote chain ID
    pub(crate) async fn ensure_chain_id_is_correct(&self) -> Result<(), EnsureChainIdError> {
        let remote_chain_id = self
            .get_sequencer_chain_id()
            .await
            .map_err(EnsureChainIdError::GetChainId)?;
        if remote_chain_id.as_str() != self.sequencer_chain_id {
            return Err(EnsureChainIdError::WrongChainId {
                expected: self.sequencer_chain_id.clone(),
                actual: remote_chain_id,
            });
        }
        Ok(())
    }

    /// Fetch chain id from the sequencer client
    async fn get_sequencer_chain_id(
        &self,
    ) -> Result<tendermint::chain::Id, sequencer_client::tendermint_rpc::Error> {
        let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
            .exponential_backoff(Duration::from_millis(100))
            .max_delay(Duration::from_secs(20))
            .on_retry(
                |attempt: u32,
                 next_delay: Option<Duration>,
                 error: &sequencer_client::tendermint_rpc::Error| {
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
        let client_genesis: tendermint::Genesis =
            tryhard::retry_fn(|| self.sequencer_client.genesis())
                .with_config(retry_config)
                .await?;
        Ok(client_genesis.chain_id)
    }
}

/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(name = "get latest nonce", skip_all, fields(%address))]
async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
    metrics: &Metrics,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
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
        metrics.increment_nonce_fetch_count();
        async move { client.get_latest_nonce(address).await.map(|rsp| rsp.nonce) }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting latest nonce from sequencer after 1024 attempts");

    metrics.record_nonce_fetch_latency(start.elapsed());

    res
}
/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(
    name = "submit signed transaction",
    skip_all,
    fields(
        nonce = tx.nonce(),
        transaction.hash = hex::encode(sha256(&tx.to_raw().encode_to_vec())),
    )
)]
async fn submit_tx(
    client: sequencer_client::HttpClient,
    tx: SignedTransaction,
    metrics: &Metrics,
) -> eyre::Result<tx_sync::Response> {
    let nonce = tx.nonce();
    metrics.set_current_nonce(nonce);

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
                metrics.increment_sequencer_submission_failure_count();

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
        async move { client.submit_transaction_sync(tx).await }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed sending transaction after 1024 attempts");

    metrics.record_sequencer_submission_latency(start.elapsed());

    res
}

pin_project! {
    /// A future to submit a bundle to the sequencer, returning the next nonce that should be used for the next submission.
    ///
    /// The future will fetch a new nonce from the sequencer if a submission returned an `INVALID_NONCE` error code.
    ///
    /// The future will only return an error if it ultimately failed submitting a transaction due to the underlying
    /// transport failing. This can be taken as a break condition to exit the executor loop.
    ///
    /// If the sequencer returned a non-zero abci code (albeit not `INVALID_NONCE`), this future will return with
    /// that nonce it used to submit the non-zero abci code request.
    struct SubmitFut {
        client: sequencer_client::HttpClient,
        address: Address,
        chain_id: String,
        nonce: u32,
        signing_key: SigningKey,
        #[pin]
        state: SubmitState,
        bundle: SizedBundle,
        metrics: &'static Metrics,
    }
}

pin_project! {
    #[project = SubmitStateProj]
    enum SubmitState {
        NotStarted,
        WaitingForSend {
            #[pin]
            fut: Pin<Box<dyn Future<Output = eyre::Result<tx_sync::Response>> + Send>>,
        },
        WaitingForNonce {
            #[pin]
            fut: Pin<Box<dyn Future<Output = eyre::Result<u32>> + Send>>,
        }
    }
}

impl Future for SubmitFut {
    type Output = eyre::Result<u32>;

    #[allow(clippy::too_many_lines)]
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            let new_state = match this.state.project() {
                SubmitStateProj::NotStarted => {
                    let params = TransactionParams::builder()
                        .nonce(*this.nonce)
                        .chain_id(&*this.chain_id)
                        .build();
                    let tx = UnsignedTransaction {
                        actions: this.bundle.clone().into_actions(),
                        params,
                    }
                    .into_signed(this.signing_key);
                    info!(
                        nonce.actual = *this.nonce,
                        bundle = %telemetry::display::json(&SizedBundleReport(this.bundle)),
                        transaction.hash = %telemetry::display::hex(&tx.sha256_of_proto_encoding()),
                        "submitting transaction to sequencer",
                    );
                    SubmitState::WaitingForSend {
                        fut: submit_tx(this.client.clone(), tx, self.metrics).boxed(),
                    }
                }

                SubmitStateProj::WaitingForSend {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(rsp) => {
                        let tendermint::abci::Code::Err(code) = rsp.code else {
                            info!("sequencer responded with ok; submission successful");

                            this.metrics
                                .record_bytes_per_submission(this.bundle.get_size());

                            this.metrics
                                .record_txs_per_submission(this.bundle.actions_count());

                            return Poll::Ready(Ok(this
                                .nonce
                                .checked_add(1)
                                .expect("nonce should not overflow")));
                        };
                        match AbciErrorCode::from(code) {
                            AbciErrorCode::INVALID_NONCE => {
                                info!(
                                    "sequencer rejected transaction due to invalid nonce; \
                                     fetching new nonce"
                                );
                                SubmitState::WaitingForNonce {
                                    fut: get_latest_nonce(
                                        this.client.clone(),
                                        *this.address,
                                        self.metrics,
                                    )
                                    .boxed(),
                                }
                            }
                            _other => {
                                warn!(
                                    abci.code = rsp.code.value(),
                                    abci.log = rsp.log,
                                    "sequencer rejected the transaction; the bundle is likely lost",
                                );

                                this.metrics.increment_sequencer_submission_failure_count();

                                return Poll::Ready(Ok(*this.nonce));
                            }
                        }
                    }
                    Err(error) => {
                        error!(%error, "failed sending transaction to sequencer");

                        return Poll::Ready(
                            Err(error).wrap_err("failed sending transaction to sequencer"),
                        );
                    }
                },

                SubmitStateProj::WaitingForNonce {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(nonce) => {
                        *this.nonce = nonce;
                        let params = TransactionParams::builder()
                            .nonce(*this.nonce)
                            .chain_id(&*this.chain_id)
                            .build();
                        let tx = UnsignedTransaction {
                            actions: this.bundle.clone().into_actions(),
                            params,
                        }
                        .into_signed(this.signing_key);
                        info!(
                            nonce.resubmission = *this.nonce,
                            bundle = %telemetry::display::json(&SizedBundleReport(this.bundle)),
                            transaction.hash = %telemetry::display::hex(&tx.sha256_of_proto_encoding()),
                            "resubmitting transaction to sequencer with new nonce",
                        );
                        SubmitState::WaitingForSend {
                            fut: submit_tx(this.client.clone(), tx, self.metrics).boxed(),
                        }
                    }
                    Err(error) => {
                        error!(%error, "critically failed getting a new nonce from the sequencer");

                        return Poll::Ready(
                            Err(error).wrap_err("failed getting nonce from sequencer"),
                        );
                    }
                },
            };
            self.as_mut().project().state.set(new_state);
        }
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Sha256;
    Sha256::digest(data)
}
