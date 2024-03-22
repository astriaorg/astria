//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

use std::time::Duration;

use astria_eyre::eyre::{
    self,
    bail,
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
use sequencer_client::HttpClient;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    instrument,
    trace,
    warn,
};

use crate::{
    block_cache::BlockCache,
    executor,
};

mod block_stream;
mod builder;
mod client;
mod reporting;
pub(crate) use builder::Builder;
pub(crate) use client::SequencerGrpcClient;

pub(crate) struct Reader {
    executor: executor::Handle,

    sequencer_grpc_client: SequencerGrpcClient,

    sequencer_cometbft_client: HttpClient,

    sequencer_block_time: Duration,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,
}

impl Reader {
    #[instrument(skip_all, err)]
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use futures::future::FusedFuture as _;
        let Self {
            mut executor,
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
        } = self;

        let mut executor = executor
            .wait_for_init()
            .await
            .wrap_err("handle to executor failed while waiting for it being initialized")?;
        let next_expected_height = executor.next_expected_soft_height();

        let mut latest_height_stream = {
            use sequencer_client::StreamLatestHeight as _;
            sequencer_cometbft_client.stream_latest_height(sequencer_block_time)
        };

        let latest_height = match latest_height_stream.next().await {
            None => bail!("subscription to sequencer for latest heights failed immediately"),
            Some(Err(e)) => {
                return Err(e).wrap_err("first latest height from sequencer was bad");
            }
            Some(Ok(height)) => height,
        };
        let mut sequential_blocks = BlockCache::with_next_height(next_expected_height)
            .wrap_err("failed constructing sequential block cache")?;
        let mut blocks_from_heights = block_stream::BlocksFromHeightStream::new(
            executor.rollup_id(),
            next_expected_height,
            latest_height,
            sequencer_grpc_client.clone(),
        );

        // Enqueued block waiting for executor to free up. Set if the executor exhibits
        // backpressure.
        let mut enqueued_block: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();

        let reason = loop {
            select! {
                biased;

                () = shutdown.cancelled() => {
                    break Ok("received shutdown signal");
                }

                // Process block execution which was enqueued due to executor channel being full.
                res = &mut enqueued_block, if !enqueued_block.is_terminated() => {
                    match res {
                        Ok(()) => debug!("submitted enqueued block to executor, resuming normal operation"),
                        Err(err) => break Err(err).wrap_err("failed sending enqueued block to executor"),
                    }
                }

                // Skip heights that executor has already executed (e.g. firm blocks from Celestia)
                Ok(next_height) = executor.next_expected_soft_height_if_changed() => {
                    blocks_from_heights.set_next_expected_height_if_greater(next_height);
                    sequential_blocks.drop_obsolete(next_height);
                }

                // Forward the next block to executor. Enqueue if the executor channel is full.
                Some(block) = sequential_blocks.next_block(), if enqueued_block.is_terminated() => {
                    if let Err(err) = executor.try_send_soft_block(block) {
                        match err {
                            // `Closed` contains the block. Dropping it because there is no use for it.
                            executor::channel::TrySendError::Closed(_) => {
                                break Err(Report::msg("could not send block to executor because its channel was closed"));
                            }

                            executor::channel::TrySendError::NoPermits(block) => {
                                trace!("executor channel is full; scheduling block and stopping block fetch until a slot opens up");
                                enqueued_block = executor.clone().send_soft_block_owned(block).boxed().fuse();
                            }
                        }
                    }
                }

                // Pull a block from the stream and put it in the block cache.
                Some(block) = blocks_from_heights.next() => {
                    // XXX: blocks_from_heights stream uses SequencerGrpcClient::get, which has
                    // retry logic. An error here means that it could not retry or
                    // otherwise recover from a failed block fetch.
                    let block = match block
                        .wrap_err("the stream of new blocks returned a catastrophic error")
                    {
                        Err(error) => break Err(error),
                        Ok(block) => block,
                    };
                    if let Err(error) = sequential_blocks.insert(block).wrap_err("failed adding block to sequential cache") {
                        warn!(%error, "failed pushing block into cache, dropping it");
                    }
                }

                // Record the latest height of the Sequencer network, allowing `blocks_from_heights` to progress.
                Some(res) = latest_height_stream.next() => {
                    match res {
                        Ok(height) => {
                            debug!(%height, "received latest height from sequencer");
                            blocks_from_heights.set_latest_observed_height_if_greater(height);
                        }
                        Err(error) => {
                            warn!(
                                error = %Report::new(error),
                                "failed fetching latest height from sequencer; waiting until next tick",
                            );
                        }
                    }
                }
            }
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
