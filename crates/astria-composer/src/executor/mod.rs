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

use astria_core::sequencer::v1::{
    transaction::action::SequenceAction,
    AbciErrorCode,
    SignedTransaction,
    UnsignedTransaction,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use ed25519_consensus::SigningKey;
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
use secrecy::{
    ExposeSecret as _,
    SecretString,
    Zeroize as _,
};
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_sync,
    Address,
    SequencerClientExt as _,
};
use tendermint::crypto::Sha256;
use tokio::{
    select,
    sync::{
        mpsc,
        mpsc::error::SendTimeoutError,
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
use crate::executor::bundle_factory::{
    BundleFactory,
    SizedBundleReport,
};

mod bundle_factory;

#[cfg(test)]
mod tests;

const BUNDLE_DRAINING_DURATION: Duration = Duration::from_secs(16);

type StdError = dyn std::error::Error;

/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
#[derive(Debug)]
pub(super) struct Executor {
    // The status of this executor
    status: watch::Sender<Status>,
    // Channel for receiving `SequenceAction`s to be bundled.
    serialized_rollup_transactions: mpsc::Receiver<SequenceAction>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: sequencer_client::HttpClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // The sequencer address associated with the private key
    address: Address,
    // Milliseconds for bundle timer to make sure bundles are submitted at least once per block.
    block_time: tokio::time::Duration,
    // Max bytes in a sequencer action bundle
    max_bytes_per_bundle: usize,
    // Token to signal the executor to stop upon shutdown.
    shutdown_token: CancellationToken,
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

impl Drop for Executor {
    fn drop(&mut self) {
        self.sequencer_key.zeroize();
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
    pub(super) fn new(
        sequencer_url: &str,
        private_key: &SecretString,
        block_time: u64,
        max_bytes_per_bundle: usize,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<(Self, Handle)> {
        let sequencer_client = sequencer_client::HttpClient::new(sequencer_url)
            .wrap_err("failed constructing sequencer client")?;
        let (status, _) = watch::channel(Status::new());
        let mut private_key_bytes: [u8; 32] = hex::decode(private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key = SigningKey::from(private_key_bytes);
        private_key_bytes.zeroize();

        let sequencer_address = Address::from_verification_key(sequencer_key.verification_key());

        let (serialized_rollup_transaction_tx, serialized_rollup_transaction_rx) =
            tokio::sync::mpsc::channel::<SequenceAction>(256);

        Ok((
            Self {
                status,
                serialized_rollup_transactions: serialized_rollup_transaction_rx,
                sequencer_client,
                sequencer_key,
                address: sequencer_address,
                block_time: Duration::from_millis(block_time),
                max_bytes_per_bundle,
                shutdown_token,
            },
            Handle::new(serialized_rollup_transaction_tx),
        ))
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Create a future to submit a bundle to the sequencer.
    #[instrument(skip_all, fields(nonce.initial = %nonce))]
    fn submit_bundle(&self, nonce: u32, bundle: SizedBundle) -> Fuse<Instrumented<SubmitFut>> {
        SubmitFut {
            client: self.sequencer_client.clone(),
            address: self.address,
            nonce,
            signing_key: self.sequencer_key.clone(),
            state: SubmitState::NotStarted,
            bundle,
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
        let mut submission_fut: Fuse<Instrumented<SubmitFut>> = Fuse::terminated();
        let mut nonce = get_latest_nonce(self.sequencer_client.clone(), self.address)
            .await
            .wrap_err("failed getting initial nonce from sequencer")?;

        self.status.send_modify(|status| status.is_connected = true);

        let block_timer = time::sleep(self.block_time);
        tokio::pin!(block_timer);
        let mut bundle_factory = BundleFactory::new(self.max_bytes_per_bundle);

        let reset_time = || Instant::now() + self.block_time;

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
                        submission_fut = self.submit_bundle(nonce, bundle);
                    }
                }

                // receive new seq_action and bundle it
                Some(seq_action) = self.serialized_rollup_transactions.recv() => {
                    let rollup_id = seq_action.rollup_id;
                    if let Err(e) = bundle_factory.try_push(seq_action) {
                            warn!(
                                rollup_id = %rollup_id,
                                error = &e as &StdError,
                                "failed to bundle sequence action, dropping it."
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
                        submission_fut = self.submit_bundle(nonce, bundle);
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
                info!(reason, "shutting down");
            }
            Err(reason) => {
                error!(%reason, "executor exited with error");
                // we error out because of a failure to submit a bundle to the sequencer
                // we do not want to proceed with the shutdown process in this case
                return Err(eyre!(reason.to_string()));
            }
        };

        let mut bundles_to_drain: VecDeque<SizedBundle> = VecDeque::new();
        let mut bundles_drained = 0;

        info!("draining sequence actions from the executor receiver channel");

        // drain the receiver channel
        while let Ok(seq_action) = self.serialized_rollup_transactions.try_recv() {
            let rollup_id = seq_action.rollup_id;
            if let Err(e) = bundle_factory.try_push(seq_action) {
                warn!(
                    rollup_id = %rollup_id,
                    error = &e as &StdError,
                    "failed to bundle sequence action, dropping it."
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

        let shutdown_logic = async {
            // wait for the last bundle to be submitted
            if !submission_fut.is_terminated() {
                info!("waiting for the last bundle to be submitted to the sequencer");
                match submission_fut.await {
                    Ok(new_nonce) => {
                        debug!(new_nonce = new_nonce, "drained bundle successfully");
                        nonce = new_nonce;
                    }
                    Err(error) => {
                        error!(%error, "failed submitting bundle to sequencer during shutdown; \
                                aborting shutdown");
                        return Err(error);
                    }
                }
            }

            info!(
                no_of_bundles_to_drain = bundles_to_drain.len(),
                "draining remaining bundles from bundle factory"
            );
            while let Some(bundle) = bundles_to_drain.pop_front() {
                match self.submit_bundle(nonce, bundle.clone()).await {
                    Ok(new_nonce) => {
                        debug!(
                            bundle = %telemetry::display::json(&SizedBundleReport(&bundle)),
                            new_nonce = new_nonce,
                            "drained bundle successfully"
                        );
                        nonce = new_nonce;
                        bundles_drained += 1;
                    }
                    Err(error) => {
                        error!(bundle = %telemetry::display::json(&SizedBundleReport(&bundle)),
                                %error, "failed submitting bundle to sequencer during shutdown; \
                                    aborting shutdown");
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

        info!(bundles_drained, "bundles drained during shutdown");

        if !bundles_to_drain.is_empty() {
            // log all the bundles that have not been drained
            let report: Vec<SizedBundleReport> =
                bundles_to_drain.iter().map(SizedBundleReport).collect();

            warn!(
                no_of_undrained_bundles = report.len(),
                undrained_bundles = %telemetry::display::json(&report),
                "unable to drain all bundles within the allocated time"
            );
        }

        reason.map(|_| ())
    }
}

/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(name = "get latest nonce", skip_all, fields(%address))]
async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
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
    tryhard::retry_fn(|| {
        let client = client.clone();
        let span = info_span!(parent: span.clone(), "attempt get nonce");
        async move { client.get_latest_nonce(address).await.map(|rsp| rsp.nonce) }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting latest nonce from sequencer after 1024 attempts")
}
/// Queries the sequencer for the latest nonce with an exponential backoff
#[instrument(
    name = "submit signed transaction",
    skip_all,
    fields(
        nonce = tx.unsigned_transaction().nonce,
        transaction.hash = hex::encode(sha256(&tx.to_raw().encode_to_vec())),
    )
)]
async fn submit_tx(
    client: sequencer_client::HttpClient,
    tx: SignedTransaction,
) -> eyre::Result<tx_sync::Response> {
    // TODO: change to info and log tx hash (to match info log in `SubmitFut`'s response handling
    // logic)
    debug!("submitting signed transaction to sequencer");
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
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
    tryhard::retry_fn(|| {
        let client = client.clone();
        let tx = tx.clone();
        let span = info_span!(parent: span.clone(), "attempt send");
        async move { client.submit_transaction_sync(tx).await }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed sending transaction after 1024 attempts")
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
        nonce: u32,
        signing_key: SigningKey,
        #[pin]
        state: SubmitState,
        bundle: SizedBundle,
    }

    impl PinnedDrop for SubmitFut {
        fn drop(this: Pin<&mut Self>) {
            this.project().signing_key.zeroize();
        }
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

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            let new_state = match this.state.project() {
                SubmitStateProj::NotStarted => {
                    let tx = UnsignedTransaction {
                        nonce: *this.nonce,
                        actions: this.bundle.clone().into_actions(),
                    }
                    .into_signed(this.signing_key);
                    info!(
                        nonce.actual = *this.nonce,
                        bundle = %telemetry::display::json(&SizedBundleReport(this.bundle)),
                        transaction.hash = %telemetry::display::base64(&tx.sha256_of_proto_encoding()),
                        "submitting transaction to sequencer",
                    );
                    SubmitState::WaitingForSend {
                        fut: submit_tx(this.client.clone(), tx).boxed(),
                    }
                }

                SubmitStateProj::WaitingForSend {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(rsp) => {
                        let tendermint::abci::Code::Err(code) = rsp.code else {
                            info!("sequencer responded with ok; submission successful");
                            return Poll::Ready(Ok(*this.nonce + 1));
                        };
                        match AbciErrorCode::from(code) {
                            AbciErrorCode::INVALID_NONCE => {
                                info!(
                                    "sequencer rejected transaction due to invalid nonce; \
                                     fetching new nonce"
                                );
                                SubmitState::WaitingForNonce {
                                    fut: get_latest_nonce(this.client.clone(), *this.address)
                                        .boxed(),
                                }
                            }
                            _other => {
                                warn!(
                                    abci.code = rsp.code.value(),
                                    abci.log = rsp.log,
                                    "sequencer rejected the transaction; the bundle is likely lost",
                                );
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
                        let tx = UnsignedTransaction {
                            nonce: *this.nonce,
                            actions: this.bundle.clone().into_actions(),
                        }
                        .into_signed(this.signing_key);
                        info!(
                            nonce.resubmission = *this.nonce,
                            bundle = %telemetry::display::json(&SizedBundleReport(this.bundle)),
                            transaction.hash = %telemetry::display::base64(&tx.sha256_of_proto_encoding()),
                            "resubmitting transaction to sequencer with new nonce",
                        );
                        SubmitState::WaitingForSend {
                            fut: submit_tx(this.client.clone(), tx).boxed(),
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
