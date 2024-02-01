use std::{
    collections::VecDeque,
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
    sync::oneshot,
};
use tracing::{
    info,
    instrument,
    warn,
};

mod block_verifier;
use block_verifier::BlockVerifier;

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
        let celestia_start_height = executor.celestia_base_block_height();

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

        // XXX: This block cache always starts at height 1, the default value for `Height`.
        let mut sequential_blocks =
            BlockCache::with_next_height(executor.next_expected_firm_height())
                .wrap_err("failed constructing sequential block cache")?;

        let mut reconstructed_blocks = ReconstructedBlocksStream::new(
            celestia_start_height,
            latest_celestia_height,
            self.celestia_http_client.clone(),
            10,
            self.block_verifier.clone(),
            self.sequencer_namespace,
            rollup_namespace,
        );
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
                    if res.is_err() {
                        bail!("executor channel closed while waiting for it to free up");
                    }
                    // we just check the error here and drop the permit without using it.
                    // because the future is now fused the branch fetching the next block will trigger.
                    reconstructed_blocks.unpause();
                }

                Some(block) = sequential_blocks.next_block() => {
                    if let Err(err) = executor.firm_blocks().try_send(block) {
                        match err {
                            tokio::sync::mpsc::error::TrySendError::Full(block) => {
                                info!("executor channel is full; stopping block fetch until a slot opens up");
                                assert!(
                                    sequential_blocks.reschedule_block(block).is_ok(),
                                    "rescheduling the just obtained block must always work",
                                );
                                executor_full = executor.soft_blocks().clone().reserve_owned().boxed().fuse();
                                reconstructed_blocks.pause();
                            }
                            tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                bail!("exiting because executor channel is closed");
                            }
                        }
                    }
                }

                // XXX: handle subscription returning None - resubscribe? what does it mean if
                // the underlying channel is full?
                Some(header) = headers.next() => {
                    match header {
                        Ok(header) => reconstructed_blocks.extend_to_height(header.height()),
                        Err(e) => {
                            warn!(
                                error = &e as &dyn  std::error::Error,
                                "header subscription returned an error",
                            );
                        }
                    }
                }

                Some(blocks) = reconstructed_blocks.next() => {
                    for block in blocks {
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
        heights: VecDeque<CelestiaHeight>,
        greatest_seen_height: CelestiaHeight,
        in_progress: FuturesMap<CelestiaHeight, eyre::Result<Vec<ReconstructedBlock>>>,
        client: HttpClient,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
        paused: bool,
    }
}

impl ReconstructedBlocksStream {
    fn extend_to_height(&mut self, height: CelestiaHeight) {
        while self.greatest_seen_height < height {
            self.greatest_seen_height.increment();
            self.heights.push_back(self.greatest_seen_height);
        }
    }

    fn new(
        start: CelestiaHeight,
        end: CelestiaHeight,
        client: HttpClient,
        max: usize,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
    ) -> Self {
        let heights: VecDeque<_> = crate::utils::height_range_inclusive(start, end).collect();
        let greatest_seen_height = heights
            .back()
            .copied()
            .expect("height range returns at least one element; this is a bug");
        Self {
            heights,
            greatest_seen_height,
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), max),
            client,
            verifier,
            sequencer_namespace,
            rollup_namespace,
            paused: false,
        }
    }

    fn pause(&mut self) {
        self.paused = true;
    }

    fn unpause(&mut self) {
        self.paused = false;
    }
}

impl Stream for ReconstructedBlocksStream {
    type Item = Vec<ReconstructedBlock>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our queue of futures.
        if !*this.paused {
            loop {
                if let Poll::Ready(()) = this.in_progress.poll_ready_unpin(cx) {
                    let Some(height) = this.heights.pop_front() else {
                        break;
                    };
                    let res = this.in_progress.try_push(
                        height,
                        fetch_blocks_at_celestia_height(
                            this.client.clone(),
                            this.verifier.clone(),
                            height,
                            *this.sequencer_namespace,
                            *this.rollup_namespace,
                        ),
                    );
                    assert!(res.is_ok(), "we polled for readiness");
                }
            }
        }

        let (height, res) = futures::ready!(this.in_progress.poll_unpin(cx));

        // Ok branch (contains the block or a fetch error): propagate the error up
        //
        // Err branch (timeout): a fetch timing out is not a problem: we can just reschedule it.
        match res {
            Ok(Ok(blocks)) => return Poll::Ready(Some(blocks)),
            // XXX: The error is silently dropped as this relies on fetch_blocks_at_celestia_height
            //      as part of its instrumentation emitting error event.
            Ok(Err(_)) => {}
            Err(timed_out) => {
                warn!(
                    %height,
                    error = &timed_out as &dyn StdError,
                    "request for height timed out, rescheduling",
                );
                this.heights.push_front(height);
            }
        }

        // We only reach this part if the `futures::ready!` didn't short circuit, or
        // if no result was ready.
        if this.heights.is_empty() || *this.paused {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = self.in_progress.len();
        let n_heights = self.heights.len();
        let len = n_heights.saturating_add(queue_len);
        (len, Some(len))
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
    let sequencer_blobs = client
        .get_sequencer_blobs(height, sequencer_namespace)
        .await
        .wrap_err("failed to fetch sequencer data from celestia")?
        .sequencer_blobs;

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
