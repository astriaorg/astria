use std::{
    collections::BTreeMap,
    error::Error as StdError,
    future::ready,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use celestia_client::{
    celestia_namespace_v0_from_rollup_id,
    celestia_types::{
        nmt::Namespace,
        Height as CelestiaHeight,
    },
    jsonrpsee::{
        http_client::HttpClient,
        ws_client::WsClient,
    },
    CelestiaClientExt as _,
    CelestiaSequencerBlob,
};
use eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
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

#[derive(Clone, Debug)]
pub(crate) struct ReconstructedBlock {
    pub(crate) block_hash: [u8; 32],
    pub(crate) header: tendermint::block::Header,
    pub(crate) transactions: Vec<Vec<u8>>,
}

impl ReconstructedBlock {
    pub(crate) fn height(&self) -> SequencerHeight {
        self.header.height
    }
}

impl GetSequencerHeight for ReconstructedBlock {
    fn get_height(&self) -> SequencerHeight {
        self.height()
    }
}

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor: executor::Handle,

    /// The client used to subscribe to new
    celestia_http_client: HttpClient,

    /// The client used to subscribe to new
    celestia_ws_client: WsClient,

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
        use celestia_client::celestia_rpc::HeaderClient as _;

        let mut executor = self
            .executor
            .wait_for_init()
            .await
            .wrap_err("handle to executor failed while waiting for it being initialized")?;

        let rollup_namespace = celestia_namespace_v0_from_rollup_id(executor.rollup_id());

        // XXX: Add retry
        let mut headers = self
            .celestia_ws_client
            .header_subscribe()
            .await
            .wrap_err("failed to subscribe to recent celestia header")?;
        let latest_celestia_height = match headers.next().await {
            Some(Ok(header)) => header.height(),
            Some(Err(e)) => {
                return Err(e).wrap_err("subscription to celestia header returned an error");
            }
            None => bail!("celestia header subscription was terminated unexpectedly"),
        };

        debug!(height = %latest_celestia_height, "received latest height from celestia");

        // XXX: This block cache always starts at height 1, the default value for `Height`.
        let mut sequential_blocks = BlockCache::<ReconstructedBlock>::with_next_height(
            executor.next_expected_firm_height(),
        )
        .wrap_err("failed constructing sequential block cache")?;

        let mut sequencer_height_to_celestia_height =
            SequencerHeightToCelestiaHeight::new(executor.next_expected_firm_height());

        let mut block_stream = ReconstructedBlocksStream {
            greatest_permissible_celestia_height: executor.celestia_base_block_height().value()
                + executor.celestia_block_variance() as u64,
            latest_observed_celestia_height: latest_celestia_height,
            next_height: executor.celestia_base_block_height(),
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), 10),
            client: self.celestia_http_client.clone(),
            verifier: self.block_verifier.clone(),
            sequencer_namespace: self.sequencer_namespace,
            rollup_namespace,
        }
        .instrument(info_span!(
            "celestia_block_stream",
            namespace.rollup = %telemetry::display::base64(&rollup_namespace.as_bytes()),
            namespace.sequencer = %telemetry::display::base64(&self.sequencer_namespace.as_bytes()),
        ));
        let mut executor_full: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();
        loop {
            select!(
                shutdown_res = &mut self.shutdown => {
                    match shutdown_res {
                        Ok(()) => info!("received shutdown command; exiting"),
                        Err(e) => {
                            let error = &e as &(dyn std::error::Error + 'static);
                            warn!(error, "shutdown receiver dropped; exiting");
                        }
                    }
                    break;
                }

                res = &mut executor_full, if !executor_full.is_terminated() => {
                    // we just check the error here and drop the permit without using it.
                    // because the future is now fused the branch fetching the next block will trigger.
                    if res.is_err() {
                        bail!("executor channel closed while waiting for it to free up");
                    }
                }

                Some(block) = sequential_blocks.next_block(), if executor_full.is_terminated() => {
                    let height_in_block = block.height();
                    match executor.firm_blocks().try_send(block) {
                        Ok(()) => {
                            let (sequencer_height, celestia_height)
                                = sequencer_height_to_celestia_height.increment_next_height();
                            assert_eq!(height_in_block, sequencer_height);
                            let new_permissible_height = celestia_height.value() + executor.celestia_block_variance() as u64;
                            block_stream.inner_mut().set_permissible_height(new_permissible_height);
                        }
                        Err(TrySendError::Full(block)) => {
                            info!("executor channel is full; rescheduling block fetch until the channel opens up");
                            assert!(
                                sequential_blocks.reschedule_block(block).is_ok(),
                                "rescheduling the just obtained block must always work",
                            );
                            executor_full = executor.firm_blocks().clone().reserve_owned().boxed().fuse();
                        }

                        Err(TrySendError::Closed(_)) =>
                                bail!("exiting because executor channel is closed"),
                    }
                }

                // XXX: handle subscription returning None - resubscribe? what does it mean if
                // the underlying channel is full?
                Some(header) = headers.next() => {
                    match header {
                        Ok(header) =>
                            block_stream.inner_mut().record_latest_height(header.height()),

                        Err(e) => {
                            warn!(
                                error = &e as &dyn  std::error::Error,
                                "header subscription returned an error",
                            );
                        }
                    }
                }

                Some((celestia_height, blocks)) = block_stream.next() => {
                    debug!(
                        height.celestia = %celestia_height,
                        num_sequencer_blocks = blocks.len(),
                        "read sequencer blocks from celestia",
                    );
                    for block in blocks {
                        sequencer_height_to_celestia_height.insert(block.height(), celestia_height);
                        if let Err(e) = sequential_blocks.insert(block) {
                            warn!(
                                error = &e as &dyn std::error::Error,
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

pin_project! {
    struct ReconstructedBlocksStream {
        greatest_permissible_celestia_height: u64,
        latest_observed_celestia_height: CelestiaHeight,
        next_height: CelestiaHeight,

        in_progress: FuturesMap<CelestiaHeight, eyre::Result<Vec<ReconstructedBlock>>>,

        client: HttpClient,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
    }
}

impl ReconstructedBlocksStream {
    fn next_height_to_fetch(&self) -> Option<CelestiaHeight> {
        if self.next_height.value() > self.greatest_permissible_celestia_height
            || self.next_height > self.latest_observed_celestia_height
        {
            return None;
        }
        Some(self.next_height)
    }

    fn set_permissible_height(&mut self, height: u64) {
        if height < self.greatest_permissible_celestia_height {
            info!("provided permissible celestia height older than previous; ignoring it",);
        }
        self.greatest_permissible_celestia_height = height;
    }

    fn record_latest_height(&mut self, height: CelestiaHeight) {
        if height < self.latest_observed_celestia_height {
            info!("observed latest celestia height older than previous; ignoring it",);
        }
        self.latest_observed_celestia_height = height;
    }
}

impl Stream for ReconstructedBlocksStream {
    type Item = (CelestiaHeight, Vec<ReconstructedBlock>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use futures_bounded::PushError;

        // Try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Some(height) = self.as_ref().get_ref().next_height_to_fetch() {
            let this = self.as_mut().project();
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
                    *this.next_height = this.next_height.increment();
                }
            }
        }

        let this = self.as_mut().project();
        let (height, res) = futures::ready!(this.in_progress.poll_unpin(cx));

        // Ok branch (contains the block or a fetch error): propagate the error up
        //
        // Err branch (timeout): a fetch timing out is not a problem: we can just reschedule it.
        match res {
            Ok(Ok(blocks)) => return Poll::Ready(Some((height, blocks))),
            // XXX: The error is silently dropped as this relies on fetch_blocks_at_celestia_height
            //      emitting an error event as part of its as part of its instrumentation.
            Ok(Err(_)) => {}
            Err(timed_out) => {
                warn!(
                    %height,
                    error = &timed_out as &dyn StdError,
                    "request for height timed out, rescheduling",
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
        if self.as_ref().get_ref().next_height_to_fetch().is_none() {
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
    height: CelestiaHeight,
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

    // XXX: Sequencer blobs can have duplicate block hashes. We ignore this here and simply fetch
    //      process them all. We will deal with that after processing is done.
    let blocks = futures::stream::iter(sequencer_blobs)
        .then(move |blob| {
            let client = client.clone();
            let verifier = verifier.clone();
            process_sequencer_blob(client, verifier, height, rollup_namespace, blob)
        })
        .inspect_err(|err| {
            warn!(
                error = AsRef::<dyn std::error::Error>::as_ref(err),
                "failed to reconstruct block from celestia blob"
            );
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
        blob.sequencer_height = %sequencer_blob.height(),
        blob.block_hash = %telemetry::display::hex(&sequencer_blob.block_hash()),
    ),
    err,
)]
async fn process_sequencer_blob(
    client: HttpClient,
    verifier: BlockVerifier,
    height: CelestiaHeight,
    rollup_namespace: Namespace,
    sequencer_blob: CelestiaSequencerBlob,
) -> eyre::Result<ReconstructedBlock> {
    verifier
        .verify_blob(&sequencer_blob)
        .await
        .wrap_err("failed validating sequencer blob retrieved from celestia")?;
    let mut rollup_blobs = client
        .get_rollup_blobs_matching_sequencer_blob(height, rollup_namespace, &sequencer_blob)
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
        block_hash: sequencer_blob.block_hash(),
        header: sequencer_blob.header().clone(),
        transactions,
    })
}

struct SequencerHeightToCelestiaHeight {
    next_height: SequencerHeight,
    inner: BTreeMap<SequencerHeight, CelestiaHeight>,
}

impl SequencerHeightToCelestiaHeight {
    fn new(next_height: SequencerHeight) -> Self {
        Self {
            next_height,
            inner: BTreeMap::new(),
        }
    }

    fn increment_next_height(&mut self) -> (SequencerHeight, CelestiaHeight) {
        let old_height = self.next_height;
        self.next_height = self.next_height.increment();
        self.inner
            .remove_entry(&old_height)
            .expect("the sequencer height must have been recorded")
    }

    fn insert(&mut self, sequencer_height: SequencerHeight, celestia_height: CelestiaHeight) {
        if sequencer_height < self.next_height {
            return;
        }
        if let Some(prev_celestia_height) = self.inner.insert(sequencer_height, celestia_height) {
            warn!(
                height.sequencer = %sequencer_height,
                height.celestia.new = %celestia_height,
                height.celestia.old = %prev_celestia_height,
                "sequencer height was already mapped to another celestia height; dropping old",
            );
        }
    }
}
