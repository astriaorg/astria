use std::{
    cmp::max,
    sync::Arc,
    time::Duration,
};

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::SequencerBlockHeader,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use bytes::Bytes;
use celestia_rpc::HeaderClient as _;
use celestia_types::nmt::Namespace;
use futures::{
    future::{
        BoxFuture,
        Fuse,
        FusedFuture as _,
    },
    FutureExt as _,
};
use jsonrpsee::http_client::HttpClient as CelestiaClient;
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
    sync::mpsc,
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
    trace_span,
    warn,
};

use crate::{
    block_cache::GetSequencerHeight,
    executor::{
        FirmSendError,
        FirmTrySendError,
        StateIsInit,
    },
    metrics::Metrics,
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
        verify_metadata,
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
    pub(crate) header: SequencerBlockHeader,
    pub(crate) transactions: Vec<Bytes>,
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

pub(super) struct ReconstructedBlocks {
    celestia_height: u64,
    blocks: Vec<ReconstructedBlock>,
}

pub(crate) struct Reader {
    celestia_block_time: Duration,

    /// Client to fetch heights and blocks from Celestia.
    celestia_client: CelestiaClient,

    /// The channel used to send messages to the executor task.
    executor: executor::Handle,

    /// The client to get the sequencer namespace and verify blocks.
    sequencer_cometbft_client: SequencerClient,

    /// The number of requests per second that will be sent to Sequencer
    /// (usually to verify block data retrieved from Celestia blobs).
    sequencer_requests_per_second: u32,

    /// The chain ID of the Celestia network the reader should be communicating with.
    expected_celestia_chain_id: String,

    /// The chain ID of the Sequencer the reader should be communicating with.
    expected_sequencer_chain_id: String,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,

    metrics: &'static Metrics,
}

impl Reader {
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let (executor, sequencer_chain_id) = select!(
            () = self.shutdown.clone().cancelled_owned() => {
                info_span!("conductor::celestia::Reader::run_until_stopped").in_scope(||
                    info!("received shutdown signal while waiting for Celestia reader task to initialize")
                );
                return Ok(());
            }

            res = self.initialize() => {
                res.wrap_err("initialization of runtime information failed")?
            }
        );

        RunningReader::from_parts(self, executor, sequencer_chain_id)
            .wrap_err("failed entering run loop")?
            .run_until_stopped()
            .await
    }

    #[instrument(skip_all, err)]
    async fn initialize(
        &mut self,
    ) -> eyre::Result<(executor::Handle<StateIsInit>, tendermint::chain::Id)> {
        let validate_celestia_chain_id = async {
            let actual_celestia_chain_id = get_celestia_chain_id(&self.celestia_client)
                .await
                .wrap_err("failed to fetch Celestia chain ID")?;
            let expected_celestia_chain_id = &self.expected_celestia_chain_id;
            ensure!(
                self.expected_celestia_chain_id == actual_celestia_chain_id.as_str(),
                "expected Celestia chain id `{expected_celestia_chain_id}` does not match actual: \
                 `{actual_celestia_chain_id}`"
            );
            Ok(())
        };

        let wait_for_init_executor = async {
            self.executor
                .wait_for_init()
                .await
                .wrap_err("handle to executor failed while waiting for it being initialized")
        };

        let get_and_validate_sequencer_chain_id = async {
            let actual_sequencer_chain_id =
                get_sequencer_chain_id(self.sequencer_cometbft_client.clone())
                    .await
                    .wrap_err("failed to get sequencer chain ID")?;
            let expected_sequencer_chain_id = &self.expected_sequencer_chain_id;
            ensure!(
                self.expected_sequencer_chain_id == actual_sequencer_chain_id.to_string(),
                "expected Celestia chain id `{expected_sequencer_chain_id}` does not match \
                 actual: `{actual_sequencer_chain_id}`"
            );
            Ok(actual_sequencer_chain_id)
        };

        try_join!(
            validate_celestia_chain_id,
            wait_for_init_executor,
            get_and_validate_sequencer_chain_id
        )
        .map(|((), executor_init, sequencer_chain_id)| (executor_init, sequencer_chain_id))
    }
}

#[instrument(skip_all, err)]
async fn get_celestia_chain_id(
    celestia_client: &CelestiaClient,
) -> eyre::Result<celestia_tendermint::chain::Id> {
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &jsonrpsee::core::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch celestia network header info; retrying after backoff",
                );
                futures::future::ready(())
            },
        );
    let network_head = tryhard::retry_fn(|| celestia_client.header_network_head())
        .with_config(retry_config)
        .await?;
    Ok(network_head.chain_id().clone())
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
    reconstruction_tasks: JoinMap<u64, eyre::Result<ReconstructedBlocks>>,

    /// The stream of latest Celestia head heights (so that only Celestia blobs up to that height
    /// are fetched).
    latest_heights: LatestHeightStream,

    /// A block (reconstructed from Celestia blobs) that's waiting for the executor task to have
    /// capacity again. Used as a back pressure mechanism so that this task does not fetch more
    /// blobs if there is no capacity in the executor to execute them against the rollup in
    /// time.
    enqueued_block: Fuse<BoxFuture<'static, Result<u64, FirmSendError>>>,

    /// The latest observed head height of the Celestia network. Set by values read from
    /// the `latest_height` stream.
    celestia_head_height: Option<u64>,

    /// The next Celestia height that will be fetched.
    celestia_next_height: u64,

    /// The reference Celestia height. `celestia_reference_height` + `celestia_variance` = C is the
    /// maximum Celestia height up to which Celestia's blobs will be fetched.
    /// `celestia_reference_height` is initialized to the base Celestia height stored in the
    /// rollup genesis. It is later advanced to that Celestia height from which the next block
    /// is derived that will be executed against the rollup (only if greater than the current
    /// value; it will never go down).
    celestia_reference_height: u64,

    /// `celestia_variance` + `celestia_reference_height` define the maximum Celestia height from
    /// Celestia blobs can be fetched. Set once during initialization to the value stored in
    /// the rollup genesis.
    celestia_variance: u64,

    /// The rollup ID of the rollup that conductor is driving. Set once during initialization to
    /// the value stored in the
    rollup_id: RollupId,

    /// The Celestia namespace for which rollup-specific blobs will be requested. Derived from
    /// `rollup_id`.
    rollup_namespace: Namespace,

    /// The cometbft ID of Sequencer. Set once during initialization by querying sequencer.
    sequencer_chain_id: tendermint::chain::Id,

    /// The Celestia namespace for which Sequencer header blobs will be requested. Derived from
    /// `sequencer_chain_id`.
    sequencer_namespace: Namespace,

    metrics: &'static Metrics,
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
            sequencer_requests_per_second,
            metrics,
            ..
        } = exposed_reader;
        let block_cache =
            BlockCache::with_next_height(executor.next_expected_firm_sequencer_height())
                .wrap_err("failed constructing sequential block cache")?;

        let latest_heights = stream_latest_heights(celestia_client.clone(), celestia_block_time);
        let rollup_id = executor.rollup_id();
        let rollup_namespace = astria_core::celestia::namespace_v0_from_rollup_id(rollup_id);
        let sequencer_namespace =
            astria_core::celestia::namespace_v0_from_sha256_of_bytes(sequencer_chain_id.as_bytes());

        let celestia_next_height = executor.celestia_base_block_height();
        let celestia_reference_height = executor.celestia_base_block_height();
        let celestia_variance = executor.celestia_block_variance();

        Ok(Self {
            block_cache,
            blob_verifier: Arc::new(
                BlobVerifier::try_new(sequencer_cometbft_client, sequencer_requests_per_second)
                    .wrap_err("failed to construct blob verifier")?,
            ),
            celestia_client,
            enqueued_block: Fuse::terminated(),
            executor,
            latest_heights,
            shutdown,
            reconstruction_tasks: JoinMap::new(),

            celestia_head_height: None,
            celestia_next_height,
            celestia_reference_height,
            celestia_variance,

            rollup_id,
            rollup_namespace,
            sequencer_chain_id,
            sequencer_namespace,
            metrics,
        })
    }

    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info_span!("conductor::celestia::RunningReader::run_until_stopped").in_scope(|| {
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
        });

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
                            trace_span!("conductor::celestia::RunningReader::run_until_stopped")
                                .in_scope(||
                            trace!("submitted enqueued block to executor, resuming normal operation"));
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

                Some((celestia_height, res)) = self.reconstruction_tasks.join_next() => {
                    match flatten(res) {
                        Ok(blocks) => self.cache_reconstructed_blocks(blocks),
                        Err(error) => break Err(error).wrap_err_with(|| format!(
                            "critically failed fetching Celestia block at height \
                            `{celestia_height}` and reconstructing sequencer block data from it"
                        )),
                    }
                }

                Some(res) = self.latest_heights.next() => {
                    self.handle_latest_height(res);
                }

            );
        };

        // XXX: explicitly setting the event message (usually implicitly set by tracing)
        let message = "shutting down";
        report_exit(reason, message)
    }

    #[instrument(skip_all)]
    fn handle_latest_height(&mut self, res: eyre::Result<u64>) {
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

    #[instrument(skip_all)]
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
        let is_capacity_in_task_set = self.reconstruction_tasks.len() < 10;

        is_next_below_head && is_next_in_window && is_capacity_in_task_set
    }

    #[instrument(skip_all)]
    fn schedule_new_blobs(&mut self) {
        let mut scheduled = vec![];
        while self.can_schedule_blobs() {
            let height = self.celestia_next_height;
            self.celestia_next_height = self.celestia_next_height.saturating_add(1);
            let task = FetchConvertVerifyAndReconstruct {
                blob_verifier: self.blob_verifier.clone(),
                celestia_client: self.celestia_client.clone(),
                celestia_height: height,
                rollup_id: self.rollup_id,
                rollup_namespace: self.rollup_namespace,
                sequencer_namespace: self.sequencer_namespace,
                executor: self.executor.clone(),
                metrics: self.metrics,
            };
            self.reconstruction_tasks.spawn(height, task.execute());
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

    #[instrument(skip_all)]
    fn forward_block_to_executor(&mut self, block: ReconstructedBlock) -> eyre::Result<()> {
        let celestia_height = block.celestia_height;
        match self.executor.try_send_firm_block(block) {
            Ok(()) => self.advance_reference_celestia_height(celestia_height),
            Err(FirmTrySendError::Channel {
                source: mpsc::error::TrySendError::Full(block),
            }) => {
                trace!(
                    "executor channel is full; rescheduling block fetch until the channel opens up"
                );
                self.enqueued_block = enqueue_block(self.executor.clone(), block).boxed().fuse();
            }

            Err(FirmTrySendError::Channel {
                source: mpsc::error::TrySendError::Closed(_),
            }) => bail!("exiting because executor channel is closed"),

            Err(FirmTrySendError::NotSet) => bail!(
                "exiting because executor was configured without firm commitments; this Celestia \
                 reader should have never been started"
            ),
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

struct FetchConvertVerifyAndReconstruct {
    blob_verifier: Arc<BlobVerifier>,
    celestia_client: CelestiaClient,
    celestia_height: u64,
    rollup_id: RollupId,
    rollup_namespace: Namespace,
    sequencer_namespace: Namespace,
    executor: executor::Handle<StateIsInit>,
    metrics: &'static Metrics,
}

impl FetchConvertVerifyAndReconstruct {
    #[instrument(skip_all, fields(
        celestia_height = self.celestia_height,
        rollup_namespace = %base64(self.rollup_namespace.as_bytes()),
        sequencer_namespace = %base64(self.sequencer_namespace.as_bytes()),
        err,
    ))]
    async fn execute(self) -> eyre::Result<ReconstructedBlocks> {
        let Self {
            blob_verifier,
            celestia_client,
            celestia_height,
            rollup_id,
            rollup_namespace,
            sequencer_namespace,
            executor,
            metrics,
        } = self;

        let new_blobs = fetch_new_blobs(
            celestia_client,
            celestia_height,
            rollup_namespace,
            sequencer_namespace,
            metrics,
        )
        .await
        .wrap_err("failed fetching blobs from Celestia")?;

        metrics.record_metadata_blobs_per_celestia_fetch(new_blobs.len_header_blobs());
        metrics.record_rollup_data_blobs_per_celestia_fetch(new_blobs.len_rollup_blobs());
        info!(
            number_of_metadata_blobs = new_blobs.len_header_blobs(),
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

        metrics.record_decoded_metadata_items_per_celestia_fetch(decoded_blobs.len_headers());
        metrics.record_decoded_rollup_data_items_per_celestia_fetch(
            decoded_blobs.len_rollup_data_entries(),
        );
        info!(
            number_of_metadata_blobs = decoded_blobs.len_headers(),
            number_of_rollup_blobs = decoded_blobs.len_rollup_data_entries(),
            "decoded Sequencer header and rollup info from raw Celestia blobs",
        );

        let verified_blobs = verify_metadata(blob_verifier, decoded_blobs, executor).await;

        metrics.record_sequencer_blocks_metadata_verified_per_celestia_fetch(
            verified_blobs.len_header_blobs(),
        );
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
            blocks: reconstructed,
        };

        metrics.record_sequencer_block_information_reconstructed_per_celestia_fetch(
            reconstructed_blocks.blocks.len(),
        );
        info!(
            number_of_final_reconstructed_blocks = reconstructed_blocks.blocks.len(),
            blocks = %json(&ReportReconstructedBlocks(&reconstructed_blocks)),
            "reconstructed block information by matching verified Sequencer header blobs to rollup blobs",
        );

        Ok(reconstructed_blocks)
    }
}

#[instrument(skip_all, err)]
async fn enqueue_block(
    executor: executor::Handle<StateIsInit>,
    block: ReconstructedBlock,
) -> Result<u64, FirmSendError> {
    let celestia_height = block.celestia_height;
    executor.send_firm_block(block).await?;
    Ok(celestia_height)
}

#[instrument(skip_all, err)]
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

#[instrument(skip_all)]
fn report_exit(exit_reason: eyre::Result<&str>, message: &str) -> eyre::Result<()> {
    match exit_reason {
        Ok(reason) => {
            info!(%reason, message);
            Ok(())
        }
        Err(reason) => {
            error!(%reason, message);
            Err(reason)
        }
    }
}
