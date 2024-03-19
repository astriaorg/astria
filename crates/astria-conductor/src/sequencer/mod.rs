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

        let mut scheduled_send: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                () = shutdown.cancelled() => {
                    info!("received shutdown signal; shutting down");
                    break 'reader_loop Ok(());
                }

                Some(block) = blocks_from_heights.next() => {
                    let block = block.wrap_err("failed getting block")?;
                    if let Err(e) = sequential_blocks.insert(block) {
                        // XXX: we could temporarily kill the subscription if we put an upper limit on the cache size
                        warn!(error = &e as &dyn std::error::Error, "failed pushing block into cache, dropping it");
                    }
                }

                Ok(next_height) = executor.next_expected_soft_height_if_changed() => {
                    blocks_from_heights.set_next_expected_height_if_greater(next_height);
                    sequential_blocks.drop_obsolete(next_height);
                }

                res = &mut scheduled_send, if !scheduled_send.is_terminated() => {
                    if res.is_err() {
                        bail!("executor channel closed while waiting for it to free up");
                    }
                }

                Some(block) = sequential_blocks.next_block(), if scheduled_send.is_terminated() => {
                    if let Err(err) = executor.try_send_soft_block(block) {
                        match err {
                            executor::channel::TrySendError::Closed(_) => {
                                bail!("executor channel is closed")
                            }
                            executor::channel::TrySendError::NoPermits(block) => {
                                trace!("executor channel is full; scheduling block and stopping block fetch until a slot opens up");
                                scheduled_send = executor.clone().send_soft_block_owned(block).boxed().fuse();
                            }
                        }
                    }
                }

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
        }
    }
}
