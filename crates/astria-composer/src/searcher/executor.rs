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

use astria_core::sequencer::v1alpha1::{
    asset::default_native_asset_id,
    transaction::{
        action::SequenceAction,
        Action,
    },
    AbciErrorCode,
    SignedTransaction,
    UnsignedTransaction,
};
use color_eyre::eyre::{
    self,
    eyre,
    Context,
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
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
    time::{
        self,
        Instant,
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

use crate::searcher::bundle_factory::{
    BundleFactory,
    BundleFactoryError,
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
    // Channel for receiving `SequenceAction`s to be bundled.
    serialized_rollup_transactions_rx: mpsc::Receiver<SequenceAction>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: sequencer_client::HttpClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // The sequencer address associated with the private key
    address: Address,
    // Milliseconds for bundle timer to make sure bundles are submitted at least once per block.
    block_time_ms: u64,
    // Max bytes in a sequencer action bundle
    max_bytes_per_bundle: usize,
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
        serialized_rollup_transactions_rx: mpsc::Receiver<SequenceAction>,
        block_time: u64,
        max_bytes_per_bundle: usize,
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
            serialized_rollup_transactions_rx,
            sequencer_client,
            sequencer_key,
            address: sequencer_address,
            block_time_ms: block_time,
            max_bytes_per_bundle,
        })
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Create a future to submit a bundle to the sequencer.
    #[instrument(skip(self), fields(nonce.initial = %nonce))]
    fn submit_bundle(&self, nonce: u32, bundle: Vec<Action>) -> Fuse<SubmitFut> {
        SubmitFut {
            client: self.sequencer_client.clone(),
            address: self.address,
            nonce,
            signing_key: self.sequencer_key.clone(),
            state: SubmitState::NotStarted,
            bundle,
        }
        .fuse()
    }

    /// Run the Executor loop, calling `process_bundle` on each bundle received from the channel.
    ///
    /// # Errors
    /// An error is returned if connecting to the sequencer fails.
    #[instrument(skip_all, fields(address = %self.address))]
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let mut submission_fut: Fuse<SubmitFut> = Fuse::terminated();
        let mut nonce = get_latest_nonce(self.sequencer_client.clone(), self.address)
            .await
            .wrap_err("failed getting initial nonce from sequencer")?;
        self.status.send_modify(|status| status.is_connected = true);

        let block_timer = time::sleep(Duration::from_millis(self.block_time_ms));
        tokio::pin!(block_timer);
        let mut bundle_factory = BundleFactory::new(self.max_bytes_per_bundle);

        loop {
            select! {
                biased;

                // process submission result and update nonce
                rsp = &mut submission_fut, if !submission_fut.is_terminated() => {
                    match rsp {
                        Ok(new_nonce) => nonce = new_nonce,
                        Err(e) => {
                            let error: &(dyn std::error::Error + 'static) = e.as_ref();
                            error!(error, "failed submitting bundle to sequencer; aborting executor");
                            break Err(e).wrap_err("failed submitting bundle to sequencer");
                        }
                    }

                    block_timer.as_mut().reset(Instant::now() + Duration::from_millis(self.block_time_ms));
                }

                bundle = future::ready(bundle_factory.pop_finished()), if submission_fut.is_terminated() => {
                    if !bundle.is_empty() {
                        submission_fut = self.submit_bundle(nonce, bundle);
                    }
                }

                // receive new seq_action and bundle it
                Some(seq_action) = self.serialized_rollup_transactions_rx.recv() => {
                    let rollup_id = seq_action.rollup_id;
                    match bundle_factory.try_push(seq_action) {
                        Ok(()) => {}
                        Err(BundleFactoryError::SequenceActionTooLarge{size, max_size}) => {
                            warn!(
                                rollup_id = %rollup_id,
                                seq_action_size = size,
                                max_size = max_size,
                                "failed to bundle sequence action: too large. sequence action is dropped."
                            );
                        }
                    }
                }

                // try to preempt current bundle if the timer has ticked without submitting the next bundle
                () = &mut block_timer, if submission_fut.is_terminated() => {
                    let bundle = bundle_factory.pop_now();
                    if !bundle.is_empty() {
                        debug!(
                            bundle_len=bundle.len(),
                            "forcing bundle submission to sequencer due to block timer"
                        );
                        submission_fut = self.submit_bundle(nonce, bundle);
                    }
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
        bundle.len = %tx.actions().len()
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use astria_core::sequencer::v1alpha1::{
        transaction::action::SequenceAction,
        RollupId,
        ROLLUP_ID_LEN,
    };
    use color_eyre::eyre;
    use once_cell::sync::Lazy;
    use prost::Message;
    use sequencer_client::SignedTransaction;
    use serde_json::json;
    use tendermint_rpc::{
        endpoint::broadcast::tx_sync,
        request,
        response,
        Id,
    };
    use tokio::{
        sync::{
            mpsc,
            watch,
        },
        time,
    };
    use tracing::debug;
    use wiremock::{
        matchers::{
            body_partial_json,
            body_string_contains,
        },
        Mock,
        MockGuard,
        MockServer,
        Request,
        ResponseTemplate,
    };

    use super::{
        Executor,
        Status,
    };
    use crate::Config;

    static TELEMETRY: Lazy<()> = Lazy::new(|| {
        if std::env::var_os("TEST_LOG").is_some() {
            let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
            telemetry::init(std::io::stdout, &filter_directives).unwrap();
        } else {
            telemetry::init(std::io::sink, "").unwrap();
        }
    });

    /// Start a mock sequencer server and mount a mock for the `accounts/nonce` query.
    async fn setup() -> (MockServer, MockGuard, Config) {
        use astria_core::generated::sequencer::v1alpha1::NonceResponse;
        Lazy::force(&TELEMETRY);
        let server = MockServer::start().await;
        let startup_guard = mount_nonce_query_mock(
            &server,
            "accounts/nonce",
            NonceResponse {
                height: 0,
                nonce: 0,
            },
        )
        .await;

        let cfg = Config {
            log: String::new(),
            api_listen_addr: "127.0.0.1:0".parse().unwrap(),
            rollups: String::new(),
            sequencer_url: server.uri(),
            private_key: "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90"
                .to_string()
                .into(),
            block_time_ms: 2000,
            max_bytes_per_bundle: 1000,
        };
        (server, startup_guard, cfg)
    }

    /// Mount a mock for the `abci_query` endpoint.
    async fn mount_nonce_query_mock(
        server: &MockServer,
        query_path: &str,
        response: impl Message,
    ) -> MockGuard {
        let expected_body = json!({
            "method": "abci_query"
        });
        let response = tendermint_rpc::endpoint::abci_query::Response {
            response: tendermint_rpc::endpoint::abci_query::AbciQuery {
                value: response.encode_to_vec(),
                ..Default::default()
            },
        };
        let wrapper = response::Wrapper::new_with_id(Id::Num(1), Some(response), None);
        Mock::given(body_partial_json(&expected_body))
            .and(body_string_contains(query_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&wrapper)
                    .append_header("Content-Type", "application/json"),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount_as_scoped(server)
            .await
    }

    /// Convert a `Request` object to a `SignedTransaction`
    fn signed_tx_from_request(request: &Request) -> SignedTransaction {
        use astria_core::generated::sequencer::v1alpha1::SignedTransaction as RawSignedTransaction;
        use prost::Message as _;

        let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
            serde_json::from_slice(&request.body)
                .expect("can't deserialize to JSONRPC wrapped tx_sync::Request");
        let raw_signed_tx = RawSignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
            .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
        let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
            .expect("can't convert raw signed tx to checked signed tx");
        debug!(?signed_tx, "sequencer mock received signed transaction");

        signed_tx
    }

    /// Deserizalizes the bytes contained in a `tx_sync::Request` to a signed sequencer transaction
    /// and verifies that the contained sequence action is in the given `expected_chain_ids` and
    /// `expected_nonces`.
    async fn mount_broadcast_tx_sync_seq_actions_mock(server: &MockServer) -> MockGuard {
        let matcher = move |request: &Request| {
            let signed_tx = signed_tx_from_request(request);
            let actions = signed_tx.actions();

            // verify all received actions are sequence actions
            actions.iter().all(|action| action.as_sequence().is_some())
        };
        let jsonrpc_rsp = response::Wrapper::new_with_id(
            Id::Num(1),
            Some(tx_sync::Response {
                code: 0.into(),
                data: vec![].into(),
                log: String::new(),
                hash: tendermint::Hash::Sha256([0; 32]),
            }),
            None,
        );

        Mock::given(matcher)
            .respond_with(ResponseTemplate::new(200).set_body_json(&jsonrpc_rsp))
            .up_to_n_times(1)
            .expect(1)
            .mount_as_scoped(server)
            .await
    }

    /// Helper to wait for the executor to connect to the mock sequencer
    async fn wait_for_startup(
        status: watch::Receiver<Status>,
        nonce_guard: MockGuard,
    ) -> eyre::Result<()> {
        // wait to receive executor status
        let mut status = status.clone();
        status.wait_for(super::Status::is_connected).await.unwrap();

        tokio::time::timeout(
            Duration::from_millis(100),
            nonce_guard.wait_until_satisfied(),
        )
        .await
        .unwrap();

        Ok(())
    }

    /// Test to check that the executor sends a signed transaction to the sequencer as soon as it
    /// receives a `SequenceAction` that fills it beyond its `max_bundle_size`.
    #[tokio::test]
    async fn full_bundle() {
        // set up the executor, channel for writing seq actions, and the sequencer mock
        let (sequencer, nonce_guard, cfg) = setup().await;
        let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
        let executor = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            seq_actions_rx,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
        .unwrap();
        let status = executor.subscribe();
        let _executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for sequencer to get the initial nonce request from sequencer
        wait_for_startup(status, nonce_guard).await.unwrap();

        let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

        // send two sequence actions to the executor, the first of which is large enough to fill the
        // bundle sending the second should cause the first to immediately be submitted in
        // order to make space for the second
        let seq0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0u8; cfg.max_bytes_per_bundle - ROLLUP_ID_LEN],
        };

        let seq1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1u8; 1],
        };

        // push both sequence actions to the executor in order to force the full bundle to be sent
        seq_actions_tx.send(seq0.clone()).await.unwrap();
        seq_actions_tx.send(seq1.clone()).await.unwrap();

        // wait for the mock sequencer to receive the signed transaction
        tokio::time::timeout(
            Duration::from_millis(100),
            response_guard.wait_until_satisfied(),
        )
        .await
        .unwrap();

        // verify only one signed transaction was received by the mock sequencer
        // i.e. only the full bundle was sent and not the second one due to the block timer
        let expected_seq_actions = vec![seq0];
        let requests = response_guard.received_requests().await;
        assert_eq!(requests.len(), 1);

        // verify the expected sequence actions were received
        let signed_tx = signed_tx_from_request(&requests[0]);
        let actions = signed_tx.actions();

        assert_eq!(
            actions.len(),
            expected_seq_actions.len(),
            "received more than one action, one was supposed to fill the bundle"
        );

        for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
            let seq_action = action.as_sequence().unwrap();
            assert_eq!(
                seq_action.rollup_id, expected_seq_action.rollup_id,
                "chain id does not match. actual {:?} expected {:?}",
                seq_action.rollup_id, expected_seq_action.rollup_id
            );
            assert_eq!(
                seq_action.data, expected_seq_action.data,
                "data does not match expected data for action with rollup_id {:?}",
                seq_action.rollup_id,
            );
        }
    }

    /// Test to check that the executor sends a signed transaction to the sequencer after its
    /// `block_timer` has ticked
    #[tokio::test]
    async fn bundle_triggered_by_block_timer() {
        // set up the executor, channel for writing seq actions, and the sequencer mock
        let (sequencer, nonce_guard, cfg) = setup().await;
        let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
        let executor = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            seq_actions_rx,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
        .unwrap();
        let status = executor.subscribe();
        let _executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for sequencer to get the initial nonce request from sequencer
        wait_for_startup(status, nonce_guard).await.unwrap();

        let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

        // send two sequence actions to the executor, both small enough to fit in a single bundle
        // without filling it
        let seq0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0u8; cfg.max_bytes_per_bundle / 4],
        };

        // make sure at least one block has passed so that the executor will submit the bundle
        // despite it not being full
        time::pause();
        seq_actions_tx.send(seq0.clone()).await.unwrap();
        time::advance(Duration::from_millis(cfg.block_time_ms)).await;
        time::resume();

        // wait for the mock sequencer to receive the signed transaction
        tokio::time::timeout(
            Duration::from_millis(100),
            response_guard.wait_until_satisfied(),
        )
        .await
        .unwrap();

        // verify only one signed transaction was received by the mock sequencer
        let expected_seq_actions = vec![seq0];
        let requests = response_guard.received_requests().await;
        assert_eq!(requests.len(), 1);

        // verify the expected sequence actions were received
        let signed_tx = signed_tx_from_request(&requests[0]);
        let actions = signed_tx.actions();

        assert_eq!(
            actions.len(),
            expected_seq_actions.len(),
            "received more than one action, one was supposed to fill the bundle"
        );

        for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
            let seq_action = action.as_sequence().unwrap();
            assert_eq!(
                seq_action.rollup_id, expected_seq_action.rollup_id,
                "chain id does not match. actual {:?} expected {:?}",
                seq_action.rollup_id, expected_seq_action.rollup_id
            );
            assert_eq!(
                seq_action.data, expected_seq_action.data,
                "data does not match expected data for action with rollup_id {:?}",
                seq_action.rollup_id,
            );
        }
    }

    /// Test to check that the executor sends a signed transaction with two sequence actions to the
    /// sequencer.
    #[tokio::test]
    async fn two_seq_actions_single_bundle() {
        // set up the executor, channel for writing seq actions, and the sequencer mock
        let (sequencer, nonce_guard, cfg) = setup().await;
        let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
        let executor = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            seq_actions_rx,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
        .unwrap();
        let status = executor.subscribe();
        let _executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for sequencer to get the initial nonce request from sequencer
        wait_for_startup(status, nonce_guard).await.unwrap();

        let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

        // send two sequence actions to the executor, both small enough to fit in a single bundle
        // without filling it
        let seq0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0u8; cfg.max_bytes_per_bundle / 4],
        };

        let seq1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1u8; cfg.max_bytes_per_bundle / 4],
        };

        // make sure at least one block has passed so that the executor will submit the bundle
        // despite it not being full
        time::pause();
        seq_actions_tx.send(seq0.clone()).await.unwrap();
        seq_actions_tx.send(seq1.clone()).await.unwrap();
        time::advance(Duration::from_millis(cfg.block_time_ms)).await;
        time::resume();

        // wait for the mock sequencer to receive the signed transaction
        tokio::time::timeout(
            Duration::from_millis(100),
            response_guard.wait_until_satisfied(),
        )
        .await
        .unwrap();

        // verify only one signed transaction was received by the mock sequencer
        let expected_seq_actions = vec![seq0, seq1];
        let requests = response_guard.received_requests().await;
        assert_eq!(requests.len(), 1);

        // verify the expected sequence actions were received
        let signed_tx = signed_tx_from_request(&requests[0]);
        let actions = signed_tx.actions();

        assert_eq!(
            actions.len(),
            expected_seq_actions.len(),
            "received more than one action, one was supposed to fill the bundle"
        );

        for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
            let seq_action = action.as_sequence().unwrap();
            assert_eq!(
                seq_action.rollup_id, expected_seq_action.rollup_id,
                "chain id does not match. actual {:?} expected {:?}",
                seq_action.rollup_id, expected_seq_action.rollup_id
            );
            assert_eq!(
                seq_action.data, expected_seq_action.data,
                "data does not match expected data for action with rollup_id {:?}",
                seq_action.rollup_id,
            );
        }
    }
}
