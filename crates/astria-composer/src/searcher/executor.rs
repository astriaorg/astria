/// ! The `Executor` is responsible for:
/// - Nonce management
/// - Transaction signing
/// - Managing the connection to the sequencer
/// - Submitting transactions to the sequencer
use std::{
    pin::Pin,
    task::Poll,
    time::Duration,
};

use color_eyre::eyre::{
    self,
    eyre,
    Context,
};
use ed25519_consensus::SigningKey;
use futures::{
    future::{
        Fuse,
        FusedFuture as _,
        FutureExt as _,
    },
    ready,
    Future,
};
use pin_project_lite::pin_project;
use proto::{
    native::sequencer::v1alpha1::{
        asset::default_native_asset_id,
        Action,
        SignedTransaction,
        UnsignedTransaction,
    },
    Message as _,
};
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
use sequencer_types::abci_code::AbciCode;
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
};
use tracing::{
    debug,
    error,
    info,
    info_span,
    instrument,
    warn,
    Instrument,
    Span,
};

/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
#[derive(Debug)]
pub(super) struct Executor {
    // The status of this executor
    status: watch::Sender<Status>,
    // Channel for receiving action bundles for submission to the sequencer.
    new_bundles: mpsc::Receiver<Vec<Action>>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: sequencer_client::HttpClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // The sequencer address associated with the private key
    address: Address,
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
        new_bundles: mpsc::Receiver<Vec<Action>>,
    ) -> eyre::Result<Self> {
        let sequencer_client = sequencer_client::HttpClient::new(sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let mut private_key_bytes: [u8; 32] = hex::decode(private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key =
            SigningKey::try_from(private_key_bytes).wrap_err("failed to parse sequencer key")?;
        private_key_bytes.zeroize();

        let sequencer_address = Address::from_verification_key(sequencer_key.verification_key());

        let (status, _) = watch::channel(Status::new());

        Ok(Self {
            status,
            new_bundles,
            sequencer_client,
            sequencer_key,
            address: sequencer_address,
        })
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Run the Executor loop, calling `process_bundle` on each bundle received from the channel.
    ///
    /// # Errors
    /// An error is returned if connecting to the sequencer fails.
    #[instrument(skip_all, fields(address = %self.address))]
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        use tracing::instrument::Instrumented;
        let mut submission_fut: Fuse<Instrumented<SubmitFut>> = Fuse::terminated();
        let mut nonce = get_latest_nonce(self.sequencer_client.clone(), self.address)
            .await
            .wrap_err("failed getting initial nonce from sequencer")?;
        self.status.send_modify(|status| status.is_connected = true);
        loop {
            select! {
                biased;

                rsp = &mut submission_fut, if !submission_fut.is_terminated() => {
                    match rsp {
                        Ok(new_nonce) => nonce = new_nonce,
                        Err(e) => {
                            let error: &(dyn std::error::Error + 'static) = e.as_ref();
                            error!(error, "failed submitting bundle to sequencer; aborting executor");
                            break Err(e).wrap_err("failed submitting bundle to sequencer");
                        }
                    }
                }

                // receive new bundle for processing
                Some(bundle) = self.new_bundles.recv(), if submission_fut.is_terminated() => {
                    // TODO(https://github.com/astriaorg/astria/issues/476): Attach the hash of the
                    // bundle to the span. Linked GH issue is for agreeing on a hash for `SignedTransaction`,
                    // but both should be addressed.
                    let span =  info_span!(
                        "submit bundle",
                        nonce.initial = nonce,
                        bundle.len = bundle.len(),
                    );
                    submission_fut = SubmitFut {
                        client: self.sequencer_client.clone(),
                        address: self.address,
                        nonce,
                        signing_key: self.sequencer_key.clone(),
                        state: SubmitState::NotStarted,
                        bundle,
                    }
                    .instrument(span)
                    .fuse();
                }
            }
        }
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
        bundle: Vec<Action>,
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
                        actions: this.bundle.clone(),
                        fee_asset_id: default_native_asset_id(),
                    }
                    .into_signed(this.signing_key);
                    SubmitState::WaitingForSend {
                        fut: submit_tx(this.client.clone(), tx).boxed(),
                    }
                }

                SubmitStateProj::WaitingForSend {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(rsp) => match AbciCode::from_cometbft(rsp.code) {
                        Some(AbciCode::OK) => {
                            info!("sequencer responded with AbciCode zero; submission successful");
                            return Poll::Ready(Ok(*this.nonce + 1));
                        }
                        Some(AbciCode::INVALID_NONCE) => {
                            info!(
                                "sequencer responded with `invalid nonce` abci code; fetching new \
                                 nonce"
                            );
                            SubmitState::WaitingForNonce {
                                fut: get_latest_nonce(this.client.clone(), *this.address).boxed(),
                            }
                        }
                        _other => {
                            warn!(
                                abci.code = rsp.code.value(),
                                abci.log = rsp.log,
                                "sequencer responded with non-zero abci code; the bundle is \
                                 likely lost",
                            );
                            return Poll::Ready(Ok(*this.nonce));
                        }
                    },
                    Err(e) => {
                        let error: &(dyn std::error::Error + 'static) = e.as_ref();
                        error!(error, "failed sending transaction to sequencer");
                        return Poll::Ready(
                            Err(e).wrap_err("failed sending transaction to sequencer"),
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
                            actions: this.bundle.clone(),
                            fee_asset_id: default_native_asset_id(),
                        }
                        .into_signed(this.signing_key);
                        SubmitState::WaitingForSend {
                            fut: submit_tx(this.client.clone(), tx).boxed(),
                        }
                    }
                    Err(e) => {
                        let error: &(dyn std::error::Error + 'static) = e.as_ref();
                        error!(
                            error,
                            "critically failed getting a new nonce from the sequencer",
                        );
                        return Poll::Ready(Err(e).wrap_err("failed getting nonce from sequencer"));
                    }
                },
            };
            self.as_mut().project().state.set(new_state);
        }
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{
        Digest as _,
        Sha256,
    };
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
