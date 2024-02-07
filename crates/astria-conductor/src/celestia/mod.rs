use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    future::ready,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use celestia_client::{
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
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use futures::{
    future::BoxFuture,
    stream::{
        FuturesUnordered,
        Stream,
    },
    StreamExt as _,
};
use pin_project_lite::pin_project;
use sequencer_client::tendermint::{
    self,
    block::Height as SequencerHeight,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
};
use tracing::{
    error,
    info,
    instrument,
    warn,
};

mod block_verifier;
use block_verifier::BlockVerifier;

use crate::block_cache::{
    BlockCache,
    GetSequencerHeight,
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
    executor_channel: mpsc::UnboundedSender<ReconstructedBlock>,

    /// The client used to subscribe to new
    celestia_http_client: HttpClient,

    /// The client used to subscribe to new
    celestia_ws_client: WsClient,

    /// the first block to be fetched from celestia
    celestia_start_height: CelestiaHeight,

    /// Validates sequencer blobs read from celestia against sequencer.
    block_verifier: BlockVerifier,

    /// The celestia namespace sequencer blobs will be read from.
    sequencer_namespace: Namespace,

    /// The celestia namespace rollup blobs will be read from.
    rollup_namespace: Namespace,

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
        info!("Starting reader event loop.");

        // TODO ghi(https://github.com/astriaorg/astria/issues/470): add sync functionality to data availability reader

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
        let mut sequential_blocks = BlockCache::new();

        let mut reconstructed_blocks = ReconstructedBlockStream::new(
            self.celestia_start_height,
            latest_celestia_height,
            self.celestia_http_client.clone(),
            10,
            self.block_verifier.clone(),
            self.sequencer_namespace,
            self.rollup_namespace,
        );
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

                Some(block) = sequential_blocks.next_block() => {
                    if let Err(e) = self.executor_channel.send(block) {
                        error!(
                            error = &e as &dyn std::error::Error,
                            "failed sending block reconstructed from celestia to executor; exiting",
                        );
                        break;
                    }
                }

                // XXX: handle subscription returning None - resubscribe? what does it mean if
                // the underlying channel is full?
                Some(header) = headers.next() => {
                    match header {
                        Ok(header) => reconstructed_blocks.extend_to_height(header.height()),
                        // XXX: handle header returning an error - respawn subscription? keep going?
                        Err(e) => {
                            warn!(
                                error = &e as &dyn  std::error::Error,
                                "header subscription returned an error",
                            );
                        }
                    }
                }

                Some(block) = reconstructed_blocks.next() => {
                    if let Err(e) = sequential_blocks.insert(block) {
                        warn!(
                            error = &e as &dyn std::error::Error,
                            "failed pushing block into cache; dropping",
                        );
                    }
                }
            );
        }
        Ok(())
    }
}

pin_project! {
    struct ReconstructedBlockStream {
        heights: VecDeque<CelestiaHeight>,
        greatest_seen_height: CelestiaHeight,
        in_progress_queue: FuturesUnordered<BoxFuture<'static, eyre::Result<ReconstructedBlock>>>,
        client: HttpClient,
        max: usize,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
    }
}

impl ReconstructedBlockStream {
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
            in_progress_queue: FuturesUnordered::new(),
            client,
            max,
            verifier,
            sequencer_namespace,
            rollup_namespace,
        }
    }
}

impl Stream for ReconstructedBlockStream {
    type Item = ReconstructedBlock;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use futures::{
            ready,
            FutureExt as _,
        };
        let this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while this.in_progress_queue.len() < *this.max {
            match this.heights.pop_front() {
                Some(height) => this.in_progress_queue.push(
                    fetch_blocks_at_celestia_height(
                        this.client.clone(),
                        this.verifier.clone(),
                        height,
                        *this.sequencer_namespace,
                        *this.rollup_namespace,
                    )
                    .boxed(),
                ),
                None => break,
            }
        }

        // Attempt to pull the next value from the in_progress_queue
        let res = this.in_progress_queue.poll_next_unpin(cx);
        if let Some(val) = ready!(res) {
            match val {
                Ok(val) => return Poll::Ready(Some(val)),
                // XXX: Consider what to about the height here - push it back into the vecdeque?
                // XXX: Also - in spans is f*cked right now, fix that.
                Err(e) => warn!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed fetching celestia blobs for height, dropping height"
                ),
            }
        }

        // If more values are still coming from the stream, we're not done yet
        if this.heights.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = self.in_progress_queue.len();
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
#[instrument(skip_all, fields(
    height.celestia = %height,
    namespace.sequencer = %telemetry::display::base64(&sequencer_namespace.as_bytes()),
    namespace.rollup = %telemetry::display::base64(&rollup_namespace.as_bytes()),
))]
async fn fetch_blocks_at_celestia_height(
    client: HttpClient,
    verifier: BlockVerifier,
    height: CelestiaHeight,
    sequencer_namespace: Namespace,
    rollup_namespace: Namespace,
) -> eyre::Result<ReconstructedBlock> {
    use futures::TryStreamExt as _;
    // XXX: This object contains information about bad blobs that belonged to the
    // wrong namespace or had other issues. Consider reporting them.
    let sequencer_blobs = client
        .get_sequencer_blobs(height, sequencer_namespace)
        .await
        .wrap_err("failed to fetch sequencer data from celestia")?
        .sequencer_blobs;

    // Deduplicate blobs by the recorded block hash.
    // XXX: This assumes no two sequencer blobs with the same block hash will be written
    // to Celestia. Warn if two sequencer blobs with the same block hash are found?
    let sequencer_blobs = sequencer_blobs
        .into_iter()
        .map(|blob| (blob.block_hash(), blob))
        .collect::<HashMap<_, _>>();

    let blocks = futures::stream::iter(sequencer_blobs.into_values())
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
        .filter_map(|x| ready(x.ok()));
    tokio::pin!(blocks);
    let Some(block) = blocks.next().await else {
        bail!("no valid blobs found at height");
    };
    if blocks.next().await.is_some() {
        bail!("more than one valid sequencer blob found on for the given height");
    }
    Ok(block)
}

// FIXME: Validation performs a lookup for every sequencer blob. That seems
// unnecessary. Just fetch the info once, validate all blobs in one
// go.
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
