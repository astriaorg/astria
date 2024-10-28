//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

use std::time::Duration;

use astria_core::sequencerblock::v1::block::FilteredSequencerBlock;
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    Report,
    WrapErr as _,
};
use futures::{
    future::{
        self,
        BoxFuture,
        Fuse,
    },
    FutureExt as _,
    StreamExt as _,
};
use sequencer_client::{
    tendermint::block::Height,
    HttpClient,
    LatestHeightStream,
    StreamLatestHeight as _,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    debug_span,
    error,
    info,
    instrument,
    trace,
    warn,
    warn_span,
};

use crate::{
    block_cache::BlockCache,
    executor::{
        self,
        SoftSendError,
        SoftTrySendError,
        StateIsInit,
    },
    sequencer::block_stream::BlocksFromHeightStream,
};

mod block_stream;
mod builder;
mod client;
mod reporting;
pub(crate) use builder::Builder;
pub(crate) use client::SequencerGrpcClient;

/// [`Reader`] reads Sequencer blocks and forwards them to the [`crate::Executor`] task.
///
/// The blocks are forwarded in strictly sequential order of their Sequencr heights.
/// A [`Reader`] is created with [`Builder::build`] and run with [`Reader::run_until_stopped`].
pub(crate) struct Reader {
    /// The handle for sending sequencer blocks as soft commits to the executor
    /// and checking it for the next expected height, and rollup ID associated with
    /// this instance of Conductor.
    /// Must be initialized before it can be used.
    executor: executor::Handle,

    /// The gRPC client to fetch new blocks from the Sequencer network.
    sequencer_grpc_client: SequencerGrpcClient,

    /// The cometbft client to periodically query the latest height of the Sequencer network.
    sequencer_cometbft_client: HttpClient,

    /// The duration for the Sequencer network to produce a new block (and advance its height).
    /// The reader will wait `sequencer_block_time` before querying the network for its latest
    /// height.
    sequencer_block_time: Duration,

    /// The chain ID of the sequencer network the reader should be communicating with.
    expected_sequencer_chain_id: String,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,
}

impl Reader {
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let executor = select!(
            () = self.shutdown.clone().cancelled_owned() => {
                return report_exit(Ok("received shutdown signal while waiting for Sequencer reader task to initialize"), "");
            }
            res = self.initialize() => {
                res?
            }
        );
        RunningReader::try_from_parts(self, executor)
            .wrap_err("failed entering run loop")?
            .run_until_stopped()
            .await
    }

    #[instrument(skip_all, err)]
    async fn initialize(&mut self) -> eyre::Result<executor::Handle<StateIsInit>> {
        let actual_sequencer_chain_id =
            get_sequencer_chain_id(self.sequencer_cometbft_client.clone())
                .await
                .wrap_err("failed to get chain ID from Sequencer")?;
        let expected_sequencer_chain_id = &self.expected_sequencer_chain_id;
        ensure!(
            self.expected_sequencer_chain_id == actual_sequencer_chain_id.as_str(),
            "expected chain id `{expected_sequencer_chain_id}` does not match actual: \
             `{actual_sequencer_chain_id}`"
        );

        self.executor
            .wait_for_init()
            .await
            .wrap_err("handle to executor failed while waiting for it being initialized")
    }
}

struct RunningReader {
    /// The initialized handle to the executor task.
    /// Used for sending sequencer blocks as soft commits to the executor
    /// and checking it for the next expected height, and rollup ID associated with
    /// this instance of Conductor.
    executor: executor::Handle<StateIsInit>,

    /// Caches the filtered sequencer blocks retrieved from the Sequencer.
    /// This cache will yield a block if it contains a block that matches the
    /// next expected soft block height of the executor task (as indicated by
    /// the handle).
    block_cache: BlockCache<FilteredSequencerBlock>,

    /// A stream of the latest heights observed from the Sequencer network.
    latest_height_stream: LatestHeightStream,

    /// A stream of block heights fetched from the Sequencer network up to
    /// the latest observed sequencer height (as obtained from the `latest_height_stream`) field.
    blocks_from_heights: BlocksFromHeightStream,

    /// An enqueued block waiting for executor to free up. Set if the executor exhibits
    /// backpressure.
    enqueued_block: Fuse<BoxFuture<'static, Result<(), SoftSendError>>>,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,
}

impl RunningReader {
    fn try_from_parts(
        reader: Reader,
        mut executor: executor::Handle<StateIsInit>,
    ) -> eyre::Result<Self> {
        let Reader {
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
            ..
        } = reader;

        let next_expected_height = executor.next_expected_soft_sequencer_height();

        let latest_height_stream =
            sequencer_cometbft_client.stream_latest_height(sequencer_block_time);

        let block_cache = BlockCache::with_next_height(next_expected_height)
            .wrap_err("failed constructing sequential block cache")?;

        let blocks_from_heights = BlocksFromHeightStream::new(
            executor.rollup_id(),
            next_expected_height,
            sequencer_grpc_client,
        );

        let enqueued_block: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();
        Ok(RunningReader {
            executor,
            block_cache,
            latest_height_stream,
            blocks_from_heights,
            enqueued_block,
            shutdown,
        })
    }

    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let stop_reason = self.run_loop().await;

        // XXX: explicitly setting the message (usually implicitly set by tracing)
        let message = "shutting down";
        report_exit(stop_reason, message)
    }

    async fn run_loop(&mut self) -> eyre::Result<&'static str> {
        use futures::future::FusedFuture as _;

        loop {
            select! {
                biased;

                () = self.shutdown.cancelled() => {
                    return Ok("received shutdown signal");
                }

                // Process block execution which was enqueued due to executor channel being full.
                res = &mut self.enqueued_block, if !self.enqueued_block.is_terminated() => {
                    res.wrap_err("failed sending enqueued block to executor")?;
                    debug_span!("conductor::sequencer::RunningReader::run_loop").in_scope(||
                        debug!("submitted enqueued block to executor, resuming normal operation")
                    );
                }

                // Skip heights that executor has already executed (e.g. firm blocks from Celestia)
                Ok(next_height) = self.executor.next_expected_soft_height_if_changed() => {
                    self.update_next_expected_height(next_height);
                }

                // Forward the next block to executor. Enqueue if the executor channel is full.
                Some(block) = self.block_cache.next_block(), if self.enqueued_block.is_terminated() => {
                    self.send_to_executor(block)?;
                }

                // Pull a block from the stream and put it in the block cache.
                Some(block) = self.blocks_from_heights.next() => {
                    // XXX: blocks_from_heights stream uses self::client::SequencerGrpcClient::get, which has
                    // retry logic. An error here means that it could not retry or
                    // otherwise recover from a failed block fetch.
                    let block = block.wrap_err("the stream of new blocks returned a catastrophic error")?;
                    if let Err(error) = self.block_cache.insert(block) {
                        warn_span!("conductor::sequencer::RunningReader::run_loop").in_scope(||
                            warn!(%error, "failed pushing block into sequential cache, dropping it")
                        );
                    }
                }

                // Record the latest height of the Sequencer network, allowing `blocks_from_heights` to progress.
                Some(res) = self.latest_height_stream.next() => {
                    self.handle_latest_height(res);
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn handle_latest_height(&mut self, res: Result<Height, tendermint_rpc::Error>) {
        match res {
            Ok(height) => {
                debug!(%height, "received latest height from sequencer");
                self.blocks_from_heights
                    .set_latest_observed_height_if_greater(height);
            }
            Err(error) => {
                warn!(
                    error = %Report::new(error),
                    "failed fetching latest height from sequencer; waiting until next tick",
                );
            }
        }
    }

    /// Sends `block` to the executor task.
    ///
    /// Enqueues the block is the channel to the executor is full, sending it once
    /// it frees up.
    fn send_to_executor(&mut self, block: FilteredSequencerBlock) -> eyre::Result<()> {
        if let Err(err) = self.executor.try_send_soft_block(block) {
            match err {
                SoftTrySendError::Channel {
                    source,
                } => match *source {
                    executor::channel::TrySendError::Closed(_) => {
                        bail!("could not send block to executor because its channel was closed");
                    }

                    executor::channel::TrySendError::NoPermits(block) => {
                        trace!(
                            "executor channel is full; scheduling block and stopping block fetch \
                             until a slot opens up"
                        );
                        self.enqueued_block = self
                            .executor
                            .clone()
                            .send_soft_block_owned(block)
                            .boxed()
                            .fuse();
                    }
                },

                SoftTrySendError::NotSet => {
                    bail!(
                        "conductor was configured without soft commitments; the sequencer reader \
                         task should have never been started",
                    );
                }
            }
        }
        Ok(())
    }

    /// Updates the next expected height to forward to the executor.
    ///
    /// This will all older heights from the cache and advance the stream of blocks
    /// so that blocks older than `next_height` will not be fetched.
    ///
    /// Already in-flight fetches will still run their course but be rejected by
    /// the block cache.
    fn update_next_expected_height(&mut self, next_height: Height) {
        self.blocks_from_heights
            .set_next_expected_height_if_greater(next_height);
        self.block_cache.drop_obsolete(next_height);
    }
}

#[instrument(skip_all)]
fn report_exit(reason: eyre::Result<&str>, message: &str) -> eyre::Result<()> {
    match reason {
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

#[instrument(skip_all, err)]
async fn get_sequencer_chain_id(
    client: sequencer_client::HttpClient,
) -> eyre::Result<tendermint::chain::Id> {
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
