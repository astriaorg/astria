use std::{
    future::ready,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use celestia_client::{
    celestia_namespace_v0_from_rollup_id,
    celestia_rpc::{
        self,
        Client,
    },
    celestia_types::{
        nmt::Namespace,
        ExtendedHeader,
    },
    jsonrpsee::{
        core::{
            client::Subscription,
            Error as JrpcError,
        },
        http_client::HttpClient,
        ws_client::WsClient,
    },
    CelestiaClientExt as _,
    CelestiaSequencerBlob,
};
use futures::{
    future::{
        self,
        BoxFuture,
        Fuse,
        FusedFuture as _,
    },
    stream::Stream,
    FutureExt as _,
    StreamExt as _,
};
use futures_bounded::FuturesMap;
use pin_project_lite::pin_project;
use sequencer_client::tendermint::{
    self,
    block::Height as SequencerHeight,
};
use tokio::{
    select,
    sync::{
        mpsc::error::TrySendError,
        oneshot,
    },
};
use tracing::{
    debug,
    error,
    info,
    info_span,
    instrument,
    trace,
    warn,
};

mod block_verifier;
use block_verifier::BlockVerifier;
use tracing_futures::Instrument;

use crate::{
    block_cache::{
        BlockCache,
        GetSequencerHeight,
    },
    executor,
};
mod builder;

type StdError = dyn std::error::Error;

#[derive(Clone, Debug)]
pub(crate) struct ReconstructedBlock {
    pub(crate) block_hash: [u8; 32],
    pub(crate) header: tendermint::block::Header,
    pub(crate) transactions: Vec<Vec<u8>>,
    pub(crate) celestia_height: u64,
}

impl ReconstructedBlock {
    pub(crate) fn sequencer_height(&self) -> SequencerHeight {
        self.header.height
    }
}

impl GetSequencerHeight for ReconstructedBlock {
    fn get_height(&self) -> SequencerHeight {
        self.sequencer_height()
    }
}

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor: executor::Handle,

    // The HTTP endpoint to fetch celestia blocks.
    celestia_http_endpoint: String,

    // The WS endpoint to subscribe to the latest celestia headers and read heights.
    celestia_ws_endpoint: String,

    // The bearer token to authenticate with the celestia node.
    celestia_auth_token: String,

    /// Validates sequencer blobs read from celestia against sequencer.
    block_verifier: BlockVerifier,

    /// The celestia namespace sequencer blobs will be read from.
    sequencer_namespace: Namespace,

    /// The channel to listen for shutdown signals.
    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    pub(super) fn builder() -> builder::ReaderBuilder {
        builder::ReaderBuilder::new()
    }

    #[instrument(skip(self))]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let mut executor = self
            .executor
            .wait_for_init()
            .await
            .wrap_err("handle to executor failed while waiting for it being initialized")?;

        let rollup_id = executor.rollup_id();
        let initial_expected_sequencer_height = executor.next_expected_firm_height();
        let initial_celestia_height = executor.celestia_base_block_height();
        let celestia_variance = executor.celestia_block_variance();
        let rollup_namespace = celestia_namespace_v0_from_rollup_id(rollup_id);

        debug!(
            %rollup_id,
            %initial_expected_sequencer_height,
            %initial_celestia_height,
            celestia_variance,
            "setting up celestia reader",
        );

        let (mut _wsclient, mut headers) =
            subscribe_to_celestia_headers(&self.celestia_ws_endpoint, &self.celestia_auth_token)
                .await
                .wrap_err("failed to subscribe to celestia headers")?;

        let latest_celestia_height = match headers.next().await {
            Some(Ok(header)) => header.height(),
            Some(Err(e)) => {
                return Err(e).wrap_err("subscription to celestia header returned an error");
            }
            None => bail!("celestia header subscription was terminated unexpectedly"),
        };

        debug!(height = %latest_celestia_height, "received latest height from celestia");

        // XXX: This block cache always starts at height 1, the default value for `Height`.
        let mut sequential_blocks =
            BlockCache::<ReconstructedBlock>::with_next_height(initial_expected_sequencer_height)
                .wrap_err("failed constructing sequential block cache")?;

        let http_client =
            connect_to_celestia(&self.celestia_http_endpoint, &self.celestia_auth_token)
                .await
                .wrap_err("failed to connect to the Celestia node HTTP RPC")?;
        let mut block_stream = ReconstructedBlocksStream {
            track_heights: TrackHeights {
                reference_height: initial_celestia_height.value(),
                variance: celestia_variance,
                last_observed: latest_celestia_height.value(),
                next_height: initial_celestia_height.value(),
            },
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), 10),
            client: http_client,
            verifier: self.block_verifier.clone(),
            sequencer_namespace: self.sequencer_namespace,
            rollup_namespace,
        }
        .instrument(info_span!(
            "celestia_block_stream",
            namespace.rollup = %telemetry::display::base64(&rollup_namespace.as_bytes()),
            namespace.sequencer = %telemetry::display::base64(&self.sequencer_namespace.as_bytes()),
        ));

        let mut scheduled_block: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();
        let mut resubscribing = Fuse::terminated();
        loop {
            select!(
                shutdown_res = &mut self.shutdown => {
                    match shutdown_res {
                        Ok(()) => info!("received shutdown command; exiting"),
                        Err(error) => {
                            warn!(%error, "shutdown receiver dropped; exiting");
                        }
                    }
                    break;
                }

                res = &mut scheduled_block, if !scheduled_block.is_terminated() => {
                    if res.is_err() {
                        bail!("executor channel closed while waiting for it to free up");
                    }
                }

                Some(block) = sequential_blocks.next_block(), if scheduled_block.is_terminated() => {
                    let celestia_height = block.celestia_height;
                    match executor.try_send_firm_block(block) {
                        Ok(()) => {
                            block_stream.inner_mut().update_reference_height_if_greater(celestia_height);
                        }
                        Err(TrySendError::Full(block)) => {
                            trace!("executor channel is full; rescheduling block fetch until the channel opens up");
                            scheduled_block = executor.clone().send_firm_block(block).boxed().fuse();
                        }

                        Err(TrySendError::Closed(_)) => bail!("exiting because executor channel is closed"),
                    }
                }

                new_subscription = &mut resubscribing, if !resubscribing.is_terminated() => {
                    match new_subscription {
                        Ok(new_subscription) => (_wsclient, headers) = new_subscription,
                        Err(e) => return Err(e).wrap_err("resubscribing to celestia headers ultimately failed"),
                    }
                }

                maybe_header = headers.next(), if resubscribing.is_terminated() => {
                    let mut resubscribe = false;
                    match maybe_header {
                        Some(Ok(header)) => {
                            if !block_stream.inner_mut().update_latest_observed_height_if_greater(header.height().value()) {
                                info!("received a new Celestia header, but the height therein was already seen");
                        }}

                        Some(Err(JrpcError::ParseError(e))) => {
                            warn!(
                                error = &e as &StdError,
                                "failed to parse return value of header subscription",
                            );
                        }

                        Some(Err(e)) => {
                            warn!(
                                error = &e as &StdError,
                                "Celestia header subscription failed, resubscribing",
                            );
                            resubscribe = true;
                        }

                        None => {
                            warn!("Celestia header subscription is unexpectedly exhausted, resubscribing");
                            resubscribe = true;
                        }
                    }
                    if resubscribe {
                        resubscribing = subscribe_to_celestia_headers(
                            &self.celestia_ws_endpoint,
                            &self.celestia_auth_token,
                        ).boxed().fuse();
                    }
                }

                Some((celestia_height, blocks)) = block_stream.next() => {
                    debug!(
                        height.celestia = %celestia_height,
                        num_sequencer_blocks = blocks.len(),
                        "read sequencer blocks from celestia",
                    );
                    for block in blocks {
                        if let Err(e) = sequential_blocks.insert(block) {
                            warn!(
                                error = &e as &StdError,
                                "failed pushing block into cache; dropping",
                            );
                        }
                    }
                }
            );
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TrackHeights {
    reference_height: u64,
    variance: u32,
    last_observed: u64,
    next_height: u64,
}

impl TrackHeights {
    fn next_height_to_fetch(&self) -> Option<u64> {
        let max_permissible = self.reference_height.saturating_add(self.variance.into());
        if self.next_height < max_permissible && self.next_height <= self.last_observed {
            Some(self.next_height)
        } else {
            None
        }
    }

    fn increment_next(&mut self) {
        self.next_height += 1;
    }

    fn update_reference_height_if_greater(&mut self, height: u64) -> bool {
        let updated = self.reference_height < height;
        if updated {
            self.reference_height = height;
        }
        updated
    }

    fn update_latest_observed_height_if_greater(&mut self, height: u64) -> bool {
        let updated = self.last_observed < height;
        if updated {
            self.last_observed = height;
        }
        updated
    }
}

pin_project! {
    struct ReconstructedBlocksStream {
        track_heights: TrackHeights,

        in_progress: FuturesMap<u64, eyre::Result<Vec<ReconstructedBlock>>>,

        client: HttpClient,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
    }
}

impl ReconstructedBlocksStream {
    fn update_reference_height_if_greater(&mut self, height: u64) -> bool {
        self.track_heights
            .update_reference_height_if_greater(height)
    }

    fn update_latest_observed_height_if_greater(&mut self, height: u64) -> bool {
        self.track_heights
            .update_latest_observed_height_if_greater(height)
    }
}

impl Stream for ReconstructedBlocksStream {
    type Item = (u64, Vec<ReconstructedBlock>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use futures_bounded::PushError;

        let this = self.project();
        // Try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Some(height) = this.track_heights.next_height_to_fetch() {
            match this.in_progress.try_push(
                height,
                fetch_blocks_at_celestia_height(
                    this.client.clone(),
                    this.verifier.clone(),
                    height,
                    *this.sequencer_namespace,
                    *this.rollup_namespace,
                ),
            ) {
                Err(PushError::BeyondCapacity(_)) => {
                    break;
                }
                Err(PushError::Replaced(_)) => {
                    error!(
                        %height,
                        "scheduled to fetch blocks, but a fetch for the same height was already in-flight",
                    );
                }
                Ok(()) => {
                    debug!(height = %height, "scheduled fetch of blocks");
                    this.track_heights.increment_next();
                }
            }
        }

        let (height, res) = futures::ready!(this.in_progress.poll_unpin(cx));

        // Ok branch (contains the block or a fetch error): propagate the error up
        //
        // Err branch (timeout): a fetch timing out is not a problem: we can just reschedule it.
        match res {
            Ok(Ok(blocks)) => return Poll::Ready(Some((height, blocks))),
            // XXX: The error is silently dropped as this relies on fetch_blocks_at_celestia_height
            //      emitting an error event as part of its as part of its instrumentation.
            Ok(Err(_)) => {}
            Err(timeout) => {
                warn!(%height, error = %timeout, "request for height timed out, rescheduling",
                );
                let res = {
                    this.in_progress.try_push(
                        height,
                        fetch_blocks_at_celestia_height(
                            this.client.clone(),
                            this.verifier.clone(),
                            height,
                            *this.sequencer_namespace,
                            *this.rollup_namespace,
                        ),
                    )
                };
                assert!(
                    res.is_ok(),
                    "there must be space in the map after a future timed out"
                );
            }
        }

        // We only reach this part if `futures::ready!` above didn't short circuit and
        // if no result was ready.
        if this.track_heights.next_height_to_fetch().is_none() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.in_progress.len(), None)
    }
}

/// Fetches all blob data for the desired rollup at celestia `height`.
///
/// Performs the following operations:
/// 1. retrieves sequencer blobs at `height` matching `sequencer_namespace`;
/// 2. verifies the sequencer blobs against sequencer, dropping all blobs that failed verification;
/// 3. retrieves all rollup blobs at `height` matching `rollup_namespace` and the block hash stored
///    in the sequencer blob.
#[instrument(
    skip_all,
    fields(
        height.celestia = %height,
        namespace.sequencer = %telemetry::display::base64(&sequencer_namespace.as_bytes()),
        namespace.rollup = %telemetry::display::base64(&rollup_namespace.as_bytes()),
    ),
    err
)]
async fn fetch_blocks_at_celestia_height(
    client: HttpClient,
    verifier: BlockVerifier,
    height: u64,
    sequencer_namespace: Namespace,
    rollup_namespace: Namespace,
) -> eyre::Result<Vec<ReconstructedBlock>> {
    use futures::TryStreamExt as _;
    use tracing_futures::Instrument as _;
    // XXX: Define the error conditions for which fetching the blob should be rescheduled.
    // XXX: This object contains information about bad blobs that belonged to the
    // wrong namespace or had other issues. Consider reporting them.
    let sequencer_blobs = match client
        .get_sequencer_blobs(height, sequencer_namespace)
        .await
    {
        Err(e) if celestia_client::is_blob_not_found(&e) => {
            info!("no sequencer blobs found");
            return Ok(vec![]);
        }
        Err(e) => return Err(e).wrap_err("failed to fetch sequencer data from celestia"),
        Ok(response) => response.sequencer_blobs,
    };

    // FIXME(https://github.com/astriaorg/astria/issues/729): Sequencer blobs can have duplicate block hashes.
    // We ignore this here and handle that in downstream processing (the sequential cash will reject
    // the second blob), but we should probably do more reporting on this.
    let blocks = futures::stream::iter(sequencer_blobs)
        .then(move |blob| {
            let client = client.clone();
            let verifier = verifier.clone();
            process_sequencer_blob(client, verifier, height, rollup_namespace, blob)
        })
        .inspect_err(|error| {
            warn!(%error, "failed to reconstruct block from celestia blob");
        })
        .filter_map(|x| ready(x.ok()))
        .in_current_span()
        .collect()
        .await;
    Ok(blocks)
}

// FIXME: Validation performs a lookup for every sequencer blob at the same height.
//        That's unnecessary: just get the validation info once for each height and
//        validate all blocks at the same height in one go.
#[instrument(
    skip_all,
    fields(
        blob.sequencer_height = sequencer_blob.height().value(),
        blob.block_hash = %telemetry::display::hex(&sequencer_blob.block_hash()),
    ),
    err,
)]
async fn process_sequencer_blob(
    client: HttpClient,
    verifier: BlockVerifier,
    celestia_height: u64,
    rollup_namespace: Namespace,
    sequencer_blob: CelestiaSequencerBlob,
) -> eyre::Result<ReconstructedBlock> {
    verifier
        .verify_blob(&sequencer_blob)
        .await
        .wrap_err("failed validating sequencer blob retrieved from celestia")?;
    let mut rollup_blobs = client
        .get_rollup_blobs_matching_sequencer_blob(
            celestia_height,
            rollup_namespace,
            &sequencer_blob,
        )
        .await
        .wrap_err("failed fetching rollup blobs from celestia")?;
    ensure!(
        rollup_blobs.len() <= 1,
        "received more than one celestia rollup blob for the given namespace and height"
    );
    let transactions = rollup_blobs
        .pop()
        .map(|blob| blob.into_unchecked().transactions)
        .unwrap_or_default();
    Ok(ReconstructedBlock {
        celestia_height,
        block_hash: sequencer_blob.block_hash(),
        header: sequencer_blob.header().clone(),
        transactions,
    })
}

#[instrument(err)]
async fn subscribe_to_celestia_headers(
    endpoint: &str,
    token: &str,
) -> eyre::Result<(WsClient, Subscription<ExtendedHeader>)> {
    use celestia_client::celestia_rpc::HeaderClient as _;

    async fn connect(endpoint: &str, token: &str) -> Result<WsClient, celestia_rpc::Error> {
        let Client::Ws(client) = Client::new(endpoint, Some(token)).await? else {
            panic!("expected a celestia Websocket client but got a HTTP client");
        };
        Ok(client)
    }

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(5))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    %error,
                    "attempt to connect to subscribe to Celestia headers failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    tryhard::retry_fn(|| async move {
        let client = connect(endpoint, token)
            .await
            .wrap_err("failed to connect to Celestia Websocket RPC")?;
        let headers = client
            .header_subscribe()
            .await
            .wrap_err("failed to subscribe to Celestia headers")?;
        Ok((client, headers))
    })
    .with_config(retry_config)
    .await
    .wrap_err("retry attempts exhausted; bailing")
}

#[instrument(err)]
async fn connect_to_celestia(endpoint: &str, token: &str) -> eyre::Result<HttpClient> {
    async fn connect(endpoint: &str, token: &str) -> Result<HttpClient, celestia_rpc::Error> {
        let Client::Http(client) = Client::new(endpoint, Some(token)).await? else {
            panic!("expected a celestia HTTP client but got a Websocket client");
        };
        Ok(client)
    }

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(5))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &celestia_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    %error,
                    "attempt to connect to Celestia HTTP RPC failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    tryhard::retry_fn(|| connect(endpoint, token))
        .with_config(retry_config)
        .await
        .wrap_err("retry attempts exhausted; bailing")
}
