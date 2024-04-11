use std::{
    future::ready,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use astria_core::sequencerblock::v1alpha1::block::SequencerBlockHeader;
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use celestia_client::{
    celestia_namespace_v0_from_rollup_id,
    celestia_types::nmt::Namespace,
    jsonrpsee::http_client::HttpClient as CelestiaClient,
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
use sequencer_client::tendermint::block::Height as SequencerHeight;
use telemetry::display::{
    base64,
    json,
};
use tokio::{
    select,
    sync::mpsc::error::{
        SendError,
        TrySendError,
    },
};
use tokio_util::sync::CancellationToken;
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
mod builder;
mod latest_height_stream;
mod reporting;

use block_verifier::BlockVerifier;
pub(crate) use builder::Builder;
use latest_height_stream::LatestHeightStream;
use reporting::{
    ReportReconstructedBlocks,
    ReportSequencerHeights,
};
use tracing_futures::Instrument;

use crate::{
    block_cache::{
        BlockCache,
        GetSequencerHeight,
    },
    executor,
};

type StdError = dyn std::error::Error;

struct ReconstructedBlocks {
    celestia_height: u64,
    sequencer_namespace: Namespace,
    rollup_namespace: Namespace,
    blocks: Vec<ReconstructedBlock>,
}

#[derive(Clone, Debug)]
pub(crate) struct ReconstructedBlock {
    pub(crate) block_hash: [u8; 32],
    pub(crate) header: SequencerBlockHeader,
    pub(crate) transactions: Vec<Vec<u8>>,
    pub(crate) celestia_height: u64,
}

impl ReconstructedBlock {
    pub(crate) fn sequencer_height(&self) -> SequencerHeight {
        self.header.height()
    }
}

impl GetSequencerHeight for ReconstructedBlock {
    fn get_height(&self) -> SequencerHeight {
        self.sequencer_height()
    }
}

pub(crate) struct Reader {
    /// Validates sequencer blobs read from celestia against sequencer.
    block_verifier: BlockVerifier,

    // Client to fetch heights and blocks from Celestia.
    celestia_client: CelestiaClient,

    // A stream of the latest Celestia heights.
    latest_celestia_heights: LatestHeightStream,

    /// The channel used to send messages to the executor task.
    executor: executor::Handle,

    /// The celestia namespace sequencer blobs will be read from.
    sequencer_namespace: Namespace,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,
}

impl Reader {
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

        let latest_celestia_height = match self.latest_celestia_heights.next().await {
            Some(Ok(height)) => height,
            Some(Err(e)) => {
                return Err(e).wrap_err("subscription to celestia header returned an error");
            }
            None => bail!("celestia header subscription was terminated unexpectedly"),
        };

        debug!(
            height = latest_celestia_height,
            "received latest height from celestia"
        );

        // XXX: This block cache always starts at height 1, the default value for `Height`.
        let mut sequential_blocks =
            BlockCache::<ReconstructedBlock>::with_next_height(initial_expected_sequencer_height)
                .wrap_err("failed constructing sequential block cache")?;

        let mut block_stream = ReconstructedBlocksStream {
            track_heights: TrackHeights {
                reference_height: initial_celestia_height.value(),
                variance: celestia_variance,
                last_observed: latest_celestia_height,
                next_height: initial_celestia_height.value(),
            },
            // NOTE: Gives Celestia 600 seconds to respond. This seems reasonable because we need to
            // 1. fetch all sequencer header blobs, 2. fetch the rollup blobs, 3. verify the rollup
            // blobs.
            // XXX: This should probably have explicit retry logic instead of this futures map.
            in_progress: FuturesMap::new(std::time::Duration::from_secs(600), 10),
            client: self.celestia_client.clone(),
            verifier: self.block_verifier.clone(),
            sequencer_namespace: self.sequencer_namespace,
            rollup_namespace,
        }
        .instrument(info_span!(
            "celestia_block_stream",
            namespace.rollup = %base64(&rollup_namespace.as_bytes()),
            namespace.sequencer = %base64(&self.sequencer_namespace.as_bytes()),
        ));

        // Enqueued block waiting for executor to free up. Set if the executor exhibits
        // backpressure.
        let mut enqueued_block: Fuse<BoxFuture<Result<_, SendError<ReconstructedBlock>>>> =
            future::Fuse::terminated();

        let reason = loop {
            select!(
                biased;

                () = self.shutdown.cancelled() => {
                    break Ok("received shutdown signal");
                }

                // Process block execution which was enqueued due to executor channel being full
                res = &mut enqueued_block, if !enqueued_block.is_terminated() => {
                    match res {
                        Ok(celestia_height) => {
                            block_stream.inner_mut().update_reference_height_if_greater(celestia_height);
                            debug!("submitted enqueued block to executor, resuming normal operation");
                        }
                        Err(err) => break Err(err).wrap_err("failed sending enqueued block to executor"),
                    }
                }

                // Forward the next block to executor. Enqueue if the executor channel is full.
                Some(block) = sequential_blocks.next_block(), if enqueued_block.is_terminated() => {
                    let celestia_height = block.celestia_height;
                    match executor.try_send_firm_block(block) {
                        Ok(()) => {
                            block_stream.inner_mut().update_reference_height_if_greater(celestia_height);
                        }
                        Err(TrySendError::Full(block)) => {
                            trace!("executor channel is full; rescheduling block fetch until the channel opens up");
                            let executor_clone = executor.clone();
                            // must return the celestia height to update the reference height upon completion
                            enqueued_block = async move {
                                let celestia_height = block.celestia_height;
                                executor_clone.send_firm_block(block).await?;
                                Ok(celestia_height)
                            }.boxed().fuse();
                        }

                        Err(TrySendError::Closed(_)) => bail!("exiting because executor channel is closed"),
                    }
                }

                // Write the latest Celestia height to the block stream.
                Some(res) = self.latest_celestia_heights.next() => {
                    match res {
                        Ok(height) => {
                            debug!(height, "received height from Celestia");
                            if block_stream.inner_mut().update_latest_observed_height_if_greater(height)
                            && block_stream.inner().is_exhausted()
                            {
                                info!(
                                    reference_height = block_stream.inner().track_heights.reference_height(),
                                    variance = block_stream.inner().track_heights.variance(),
                                    max_permitted_height = block_stream.inner().track_heights.max_permitted(),
                                    "updated reference height, but the block stream is exhausted and won't fetch past its permitted window",
                                );
                            }
                        }
                        Err(error) => {
                            warn!(
                                %error,
                                "failed fetching latest height from sequencer; waiting until next tick",
                            );
                        }
                    }
                }

                // Pull the the next reconstructed block from the stream reading off of Celestia.
                Some(reconstructed) = block_stream.next() => {
                    for block in reconstructed.blocks {
                        if let Err(e) = sequential_blocks.insert(block) {
                            warn!(
                                error = &e as &StdError,
                                "failed pushing block into cache; dropping",
                            );
                        }
                    }
                }
            );
        };

        // XXX: explicitly setting the message (usually implicitly set by tracing)
        let message = "shutting down";
        match reason {
            Ok(reason) => {
                info!(reason, message);
                Ok(())
            }
            Err(reason) => {
                error!(%reason, message);
                Err(reason)
            }
        }
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
    fn reference_height(&self) -> u64 {
        self.reference_height
    }

    fn variance(&self) -> u32 {
        self.variance
    }

    fn max_permitted(&self) -> u64 {
        self.reference_height.saturating_add(self.variance.into())
    }

    fn next_height_to_fetch(&self) -> Option<u64> {
        if self.next_height <= self.max_permitted() && self.next_height <= self.last_observed {
            Some(self.next_height)
        } else {
            None
        }
    }

    fn increment_next_height_to_fetch(&mut self) {
        self.next_height = self
            .next_height
            .checked_add(1)
            .expect("this value should never reach u64::MAX");
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

        in_progress: FuturesMap<u64, eyre::Result<ReconstructedBlocks>>,

        client: CelestiaClient,
        verifier: BlockVerifier,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
    }
}

impl ReconstructedBlocksStream {
    fn is_exhausted(&self) -> bool {
        self.in_progress.is_empty() && self.track_heights.next_height_to_fetch().is_none()
    }

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
    type Item = ReconstructedBlocks;

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
                    this.track_heights.increment_next_height_to_fetch();
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
            //      emitting an error event as part of its instrumentation.
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
        %celestia_height,
        namespace.sequencer = %base64(&sequencer_namespace.as_bytes()),
        namespace.rollup = %base64(&rollup_namespace.as_bytes()),
    ),
    err
)]
async fn fetch_blocks_at_celestia_height(
    client: CelestiaClient,
    verifier: BlockVerifier,
    celestia_height: u64,
    sequencer_namespace: Namespace,
    rollup_namespace: Namespace,
) -> eyre::Result<ReconstructedBlocks> {
    use futures::TryStreamExt as _;
    use tracing_futures::Instrument as _;
    // XXX: Define the error conditions for which fetching the blob should be rescheduled.
    // XXX: This object contains information about bad blobs that belonged to the
    // wrong namespace or had other issues. Consider reporting them.
    let sequencer_blobs = match client
        .get_sequencer_blobs(celestia_height, sequencer_namespace)
        .await
    {
        Err(e) if celestia_client::is_blob_not_found(&e) => {
            info!("no sequencer blobs found");
            return Ok(ReconstructedBlocks {
                celestia_height,
                sequencer_namespace,
                rollup_namespace,
                blocks: vec![],
            });
        }
        Err(e) => return Err(e).wrap_err("failed to fetch sequencer data from celestia"),
        Ok(response) => response.sequencer_blobs,
    };

    debug!(
        %celestia_height,
        sequencer_heights = %json(&ReportSequencerHeights(&sequencer_blobs)),
        "received sequencer header blobs from Celestia"
    );

    // FIXME(https://github.com/astriaorg/astria/issues/729): Sequencer blobs can have duplicate block hashes.
    // We ignore this here and handle that in downstream processing (the sequential cash will reject
    // the second blob), but we should probably do more reporting on this.
    let blocks = futures::stream::iter(sequencer_blobs)
        .then(move |blob| {
            let client = client.clone();
            let verifier = verifier.clone();
            process_sequencer_blob(client, verifier, celestia_height, rollup_namespace, blob)
        })
        .inspect_err(|error| {
            warn!(%error, "failed to reconstruct block from celestia blob");
        })
        .filter_map(|x| ready(x.ok()))
        .in_current_span()
        .collect()
        .await;
    let reconstructed = ReconstructedBlocks {
        celestia_height,
        sequencer_namespace,
        rollup_namespace,
        blocks,
    };
    info!(
        blocks = %json(&ReportReconstructedBlocks(&reconstructed)),
        "received and validated reconstructed Sequencer blocks from Celestia",
    );
    Ok(reconstructed)
}

// FIXME: Validation performs a lookup for every sequencer blob at the same height.
//        That's unnecessary: just get the validation info once for each height and
//        validate all blocks at the same height in one go.
#[instrument(
    skip_all,
    fields(
        blob.sequencer_height = sequencer_blob.height().value(),
        blob.block_hash = %base64(&sequencer_blob.block_hash()),
        celestia_rollup_namespace = %base64(rollup_namespace.as_bytes()),
    ),
    err,
)]
async fn process_sequencer_blob(
    client: CelestiaClient,
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
    debug!(
        %celestia_height,
        number_of_blobs = rollup_blobs.len(),
        "received rollup blobs from Celestia"
    );
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

#[cfg(test)]
mod tests {
    use super::TrackHeights;

    #[test]
    fn next_height_within_allowed_and_observed_is_some() {
        let tracked = TrackHeights {
            reference_height: 10,
            variance: 10,
            last_observed: 20,
            next_height: 20,
        };
        assert_eq!(Some(20), tracked.next_height_to_fetch());
    }

    #[test]
    fn next_height_ahead_of_observed_is_none() {
        let tracked = TrackHeights {
            reference_height: 10,
            variance: 20,
            last_observed: 20,
            next_height: 21,
        };
        assert_eq!(None, tracked.next_height_to_fetch());
    }

    #[test]
    fn next_height_ahead_of_permitted_is_none() {
        let tracked = TrackHeights {
            reference_height: 10,
            variance: 10,
            last_observed: 30,
            next_height: 21,
        };
        assert_eq!(None, tracked.next_height_to_fetch());
    }

    #[test]
    fn incrementing_next_height_past_observed_flips_to_none() {
        let mut tracked = TrackHeights {
            reference_height: 10,
            variance: 20,
            last_observed: 20,
            next_height: 20,
        };
        assert_eq!(Some(20), tracked.next_height_to_fetch());
        tracked.increment_next_height_to_fetch();
        assert_eq!(None, tracked.next_height_to_fetch());
    }

    #[test]
    fn incrementing_next_height_past_variance_flips_to_none() {
        let mut tracked = TrackHeights {
            reference_height: 10,
            variance: 10,
            last_observed: 30,
            next_height: 20,
        };
        assert_eq!(Some(20), tracked.next_height_to_fetch());
        tracked.increment_next_height_to_fetch();
        assert_eq!(None, tracked.next_height_to_fetch());
    }
}
