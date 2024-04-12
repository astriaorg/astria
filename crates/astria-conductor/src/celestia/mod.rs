use std::{
    cmp::max,
    sync::Arc,
    time::Duration,
};

use astria_core::sequencer::v1::RollupId;
use astria_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use celestia_client::{
    celestia_types::nmt::Namespace,
    jsonrpsee::http_client::HttpClient as CelestiaClient,
};
use futures::{
    future::{
        BoxFuture,
        Fuse,
        FusedFuture as _,
    },
    FutureExt as _,
};
use sequencer_client::{
    tendermint,
    tendermint::block::Height as SequencerHeight,
    tendermint_rpc,
    HttpClient as SequencerClient,
};
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
    task::spawn_blocking,
    try_join,
};
use tokio_stream::StreamExt as _;
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    info,
    info_span,
    instrument,
    trace,
    warn,
};

use crate::{
    block_cache::GetSequencerHeight,
    executor::StateIsInit,
    utils::flatten,
};

mod block_verifier;
mod builder;
mod convert;
mod fetch;
mod latest_height_stream;
mod reconstruct;
mod reporting;
mod verify;

pub(crate) use builder::Builder;
use latest_height_stream::LatestHeightStream;
use reporting::ReportReconstructedBlocks;

use self::{
    block_verifier::ensure_commit_has_quorum,
    convert::decode_raw_blobs,
    fetch::fetch_new_blobs,
    latest_height_stream::stream_latest_heights,
    reconstruct::reconstruct_blocks_from_verified_blobs,
    verify::{
        verify_and_filter_decoded_blobs,
        BlobVerifier,
    },
};
use crate::{
    block_cache::BlockCache,
    executor,
};

/// Sequencer Block information reconstructed from Celestia blobs.
///
/// Will be forwarded to the executor as a firm block.
#[derive(Clone, Debug)]
pub(crate) struct ReconstructedBlock {
    pub(crate) celestia_height: u64,
    pub(crate) block_hash: [u8; 32],
    pub(crate) header: tendermint::block::Header,
    pub(crate) transactions: Vec<Vec<u8>>,
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

pub(super) struct ReconstructedBlocks {
    celestia_height: u64,
    sequencer_namespace: Namespace,
    rollup_namespace: Namespace,
    blocks: Vec<ReconstructedBlock>,
}

pub(crate) struct Reader {
    celestia_block_time: Duration,

    // Client to fetch heights and blocks from Celestia.
    celestia_client: CelestiaClient,

    /// The channel used to send messages to the executor task.
    executor: executor::Handle,

    /// The client to get the sequencer namespace and verify blocks.
    sequencer_cometbft_client: SequencerClient,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,
}

impl Reader {
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let (executor, sequencer_chain_id) = self
            .initialize()
            .await
            .wrap_err("intialization of runtime information failed")?;

        RunningReader::from_parts(self, executor, sequencer_chain_id)
            .wrap_err("failed entering run loop")?
            .run_until_stopped()
            .await
    }

    async fn initialize(
        &mut self,
    ) -> eyre::Result<(executor::Handle<StateIsInit>, tendermint::chain::Id)> {
        let wait_for_init_executor = async {
            self.executor
                .wait_for_init()
                .await
                .wrap_err("handle to executor failed while waiting for it being initialized")
        };

        let get_sequencer_chain_id = async {
            get_sequencer_chain_id(self.sequencer_cometbft_client.clone())
                .await
                .wrap_err("failed to get sequencer chain ID")
        };

        try_join!(wait_for_init_executor, get_sequencer_chain_id)
    }
}

struct RunningReader {
    block_cache: BlockCache<ReconstructedBlock>,

    blob_verifier: Arc<BlobVerifier>,

    // Client to fetch heights and blocks from Celestia.
    celestia_client: CelestiaClient,

    /// The channel used to send messages to the executor task.
    executor: executor::Handle<StateIsInit>,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,

    /// Tasks reconstructing Sequencer block information from Celestia blobs.
    tasks: JoinMap<u64, eyre::Result<ReconstructedBlocks>>,

    latest_heights: LatestHeightStream,

    enqueued_block: Fuse<BoxFuture<'static, Result<u64, SendError<ReconstructedBlock>>>>,

    celestia_head_height: Option<u64>,
    celestia_next_height: u64,
    celestia_reference_height: u64,
    celestia_variance: u64,

    rollup_id: RollupId,
    rollup_namespace: Namespace,

    sequencer_chain_id: tendermint::chain::Id,
    sequencer_namespace: Namespace,
}

impl RunningReader {
    fn from_parts(
        exposed_reader: Reader,
        mut executor: executor::Handle<StateIsInit>,
        sequencer_chain_id: tendermint::chain::Id,
    ) -> eyre::Result<Self> {
        let Reader {
            celestia_block_time,
            celestia_client,
            sequencer_cometbft_client,
            shutdown,
            ..
        } = exposed_reader;
        let block_cache = BlockCache::with_next_height(executor.next_expected_firm_height())
            .wrap_err("failed constructing sequential block cache")?;

        let latest_heights = stream_latest_heights(celestia_client.clone(), celestia_block_time);
        let rollup_id = executor.rollup_id();
        let rollup_namespace = celestia_client::celestia_namespace_v0_from_rollup_id(rollup_id);
        let sequencer_namespace =
            celestia_client::celestia_namespace_v0_from_bytes(sequencer_chain_id.as_bytes());

        let celestia_next_height = executor.celestia_base_block_height().value();
        let celestia_reference_height = executor.celestia_base_block_height().value();
        let celestia_variance = executor.celestia_block_variance().into();

        Ok(Self {
            block_cache,
            blob_verifier: Arc::new(BlobVerifier::new(sequencer_cometbft_client)),
            celestia_client,
            enqueued_block: Fuse::terminated(),
            executor,
            latest_heights,
            shutdown,
            tasks: JoinMap::new(),

            celestia_head_height: None,
            celestia_next_height,
            celestia_reference_height,
            celestia_variance,

            rollup_id,
            rollup_namespace,
            sequencer_chain_id,
            sequencer_namespace,
        })
    }

    #[instrument(skip(self))]
    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info!(
            initial_celestia_height = self.celestia_next_height,
            initial_max_celestia_height = self.max_permitted_celestia_height(),
            celestia_variance = self.celestia_variance,
            rollup_namespace = %base64(&self.rollup_namespace.as_bytes()),
            rollup_id = %self.rollup_id,
            sequencer_chain_id = %self.sequencer_chain_id,
            sequencer_namespace = %base64(&self.sequencer_namespace.as_bytes()),
            "starting firm block read loop",
        );

        let reason = loop {
            self.schedule_new_blobs();

            select!(
                biased;

                () = self.shutdown.cancelled() => {
                    break Ok("received shutdown signal");
                }

                res = &mut self.enqueued_block, if self.waiting_for_executor_capacity() => {
                    match res {
                        Ok(celestia_height_of_forwarded_block) => {
                            trace!("submitted enqueued block to executor, resuming normal operation");
                            self.advance_reference_celestia_height(celestia_height_of_forwarded_block);
                        }
                        Err(err) => break Err(err).wrap_err("failed sending enqueued block to executor"),
                    }
                }

                Some(block) = self.block_cache.next_block(), if !self.waiting_for_executor_capacity() => {
                    if let Err(err) = self.forward_block_to_executor(block) {
                        break Err(err);
                    }
                }

                Some((celestia_height, res)) = self.tasks.join_next() => {
                    match flatten(res) {
                        Ok(blocks) => self.cache_reconstructed_blocks(blocks),
                        Err(error) => break Err(error).wrap_err_with(|| format!(
                            "critically failed fetching Celestia block at height \
                            `{celestia_height}` and reconstructing sequencer block data from it"
                        )),
                    }
                }

                Some(res) = self.latest_heights.next() => {
                    match res {
                        Ok(height) => {
                            info!(height, "observed latest height from Celestia");
                            self.record_latest_celestia_height(height);
                        }
                        Err(error) => {
                            warn!(
                                %error,
                                "failed fetching latest height from sequencer; waiting until next tick",
                            );
                        }
                    }
                }

            );
        };

        // XXX: explicitly setting the event message (usually implicitly set by tracing)
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

    fn cache_reconstructed_blocks(&mut self, reconstructed: ReconstructedBlocks) {
        for block in reconstructed.blocks {
            let block_hash = block.block_hash;
            let celestia_height = block.celestia_height;
            let sequencer_height = block.sequencer_height().value();
            if let Err(e) = self.block_cache.insert(block) {
                warn!(
                    error = %eyre::Report::new(e),
                    source_celestia_height = celestia_height,
                    sequencer_height,
                    block_hash = %base64(&block_hash),
                    "failed pushing reconstructed block into sequential cache; dropping it",
                );
            }
        }
    }

    fn can_schedule_blobs(&self) -> bool {
        let Some(head_height) = self.celestia_head_height else {
            return false;
        };

        let is_next_below_head = self.celestia_next_height <= head_height;
        let is_next_in_window = self.celestia_next_height <= self.max_permitted_celestia_height();
        let is_capacity_in_task_set = self.tasks.len() < 10;

        is_next_below_head && is_next_in_window && is_capacity_in_task_set
    }

    fn schedule_new_blobs(&mut self) {
        let mut scheduled = vec![];
        while self.can_schedule_blobs() {
            let height = self.celestia_next_height;
            self.celestia_next_height = self.celestia_next_height.saturating_add(1);
            let task = FetchConvertAndVerify {
                blob_verifier: self.blob_verifier.clone(),
                celestia_client: self.celestia_client.clone(),
                celestia_height: height,
                rollup_id: self.rollup_id,
                rollup_namespace: self.rollup_namespace,
                sequencer_namespace: self.sequencer_namespace,
            };
            self.tasks.spawn(height, task.execute());
            scheduled.push(height);
        }
        if !scheduled.is_empty() {
            info!(
                heights = %json(&scheduled),
                "scheduled next batch of Celestia heights",
            );
        }
    }

    fn advance_reference_celestia_height(&mut self, candidate: u64) {
        let reference_height = &mut self.celestia_reference_height;
        *reference_height = max(*reference_height, candidate);
    }

    fn forward_block_to_executor(&mut self, block: ReconstructedBlock) -> eyre::Result<()> {
        let celestia_height = block.celestia_height;
        match self.executor.try_send_firm_block(block) {
            Ok(()) => self.advance_reference_celestia_height(celestia_height),
            Err(TrySendError::Full(block)) => {
                trace!(
                    "executor channel is full; rescheduling block fetch until the channel opens up"
                );
                self.enqueued_block = enqueue_block(self.executor.clone(), block).boxed().fuse();
            }

            Err(TrySendError::Closed(_)) => bail!("exiting because executor channel is closed"),
        }
        Ok(())
    }

    /// Returns the maximum permitted Celestia height given the current state.
    ///
    /// The maximum permitted Celestia height is calculated as `ref_height + 6 * variance`, with:
    ///
    /// - `ref_height` the height from which the last expected sequencer block was derived,
    /// - `variance` the `celestia_block_variance` received from the connected rollup genesis info,
    /// - and the factor 6 based on the assumption that there are up to 6 sequencer heights stored
    ///   per Celestia height.
    fn max_permitted_celestia_height(&self) -> u64 {
        max_permitted_celestia_height(self.celestia_reference_height, self.celestia_variance)
    }

    fn record_latest_celestia_height(&mut self, height: u64) {
        let head_height = self.celestia_head_height.get_or_insert(height);
        *head_height = max(*head_height, height);
    }

    fn waiting_for_executor_capacity(&self) -> bool {
        !self.enqueued_block.is_terminated()
    }
}

struct FetchConvertAndVerify {
    blob_verifier: Arc<BlobVerifier>,
    celestia_client: CelestiaClient,
    celestia_height: u64,
    rollup_id: RollupId,
    rollup_namespace: Namespace,
    sequencer_namespace: Namespace,
}

impl FetchConvertAndVerify {
    #[instrument( skip_all, fields(
        celestia_height = self.celestia_height,
        rollup_namespace = %base64(self.rollup_namespace.as_bytes()),
        sequencer_namespace = %base64(self.sequencer_namespace.as_bytes()),
    ))]
    async fn execute(self) -> eyre::Result<ReconstructedBlocks> {
        let Self {
            blob_verifier,
            celestia_client,
            celestia_height,
            rollup_id,
            rollup_namespace,
            sequencer_namespace,
        } = self;

        let new_blobs = fetch_new_blobs(
            celestia_client,
            celestia_height,
            rollup_namespace,
            sequencer_namespace,
        )
        .await
        .wrap_err("failed fetching blobs from Celestia")?;

        info!(
            number_of_header_blobs = new_blobs.len_header_blobs(),
            number_of_rollup_blobs = new_blobs.len_rollup_blobs(),
            "received new Celestia blobs"
        );

        let decode_span = info_span!("decode_blobs");
        let decoded_blobs = spawn_blocking(move || {
            decode_span
                .in_scope(|| decode_raw_blobs(new_blobs, rollup_namespace, sequencer_namespace))
        })
        .await
        .wrap_err("encountered panic while decoding raw Celestia blobs")?;

        info!(
            number_of_header_blobs = decoded_blobs.len_header_blobs(),
            number_of_rollup_blobs = decoded_blobs.len_rollup_blobs(),
            "decoded Sequencer header and rollup info from raw Celestia blobs",
        );

        let verified_blobs = verify_and_filter_decoded_blobs(blob_verifier, decoded_blobs).await;

        info!(
            number_of_verified_header_blobs = verified_blobs.len_header_blobs(),
            number_of_rollup_blobs = verified_blobs.len_rollup_blobs(),
            "verified header blobs against Sequencer",
        );

        let reconstruct_span = info_span!("reconstruct_blocks");
        let reconstructed = spawn_blocking(move || {
            reconstruct_span
                .in_scope(|| reconstruct_blocks_from_verified_blobs(verified_blobs, rollup_id))
        })
        .await
        .wrap_err("encountered panic while reconstructing blocks from verified blobs")?;

        let reconstructed_blocks = ReconstructedBlocks {
            celestia_height,
            sequencer_namespace,
            rollup_namespace,
            blocks: reconstructed,
        };

        info!(
            number_of_final_reconstructed_blocks = reconstructed_blocks.blocks.len(),
            blocks = %json(&ReportReconstructedBlocks(&reconstructed_blocks)),
            "reconstructed block information by matching verified Sequencer header blobs to rollup blobs",
        );

        Ok(reconstructed_blocks)
    }
}

async fn enqueue_block(
    executor: executor::Handle<StateIsInit>,
    block: ReconstructedBlock,
) -> Result<u64, SendError<ReconstructedBlock>> {
    let celestia_height = block.celestia_height;
    executor.send_firm_block(block).await?;
    Ok(celestia_height)
}

async fn get_sequencer_chain_id(client: SequencerClient) -> eyre::Result<tendermint::chain::Id> {
    use sequencer_client::Client as _;

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
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

    let genesis: tendermint::Genesis = tryhard::retry_fn(|| client.genesis())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get genesis info from Sequencer after a lot of attempts")?;

    Ok(genesis.chain_id)
}

fn max_permitted_celestia_height(reference: u64, variance: u64) -> u64 {
    reference.saturating_add(variance.saturating_mul(6))
}
