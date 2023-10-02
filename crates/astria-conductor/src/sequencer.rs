//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use astria_sequencer_types::SequencerBlockData;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use sequencer_client::extension_trait::NewBlocksStream;
use tokio::{
    select,
    sync::oneshot,
};
use tracing::{
    info,
    instrument,
    warn,
};

use crate::{
    executor,
    executor::ExecutorCommand,
};

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,

    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn new(
        pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
    ) -> eyre::Result<Self> {
        Ok(Self {
            executor_tx,
            pool,
            shutdown,
        })
    }

    /// Run the sequencer reader event loop
    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        use futures::{
            stream::FuturesOrdered,
            FutureExt as _,
            StreamExt as _,
            TryFutureExt as _,
        };
        use sequencer_client::SequencerClientExt as _;

        let latest_block = self
            .pool
            .get()
            .await
            .wrap_err("failed getting client from pool to fetch initial latest block")?
            .latest_sequencer_block()
            .await
            .wrap_err("failed to issue an initial request for the latest block")?;

        let mut sync_heights = {
            let pool = self.pool.clone();
            Box::pin(
                futures::stream::iter(1..latest_block.header().height.value()).then(
                    move |height| {
                        let height: u32 = height.try_into().expect(
                            "converting a u64 extract from a tendermint Height should always work",
                        );
                        let pool = pool.clone();
                        async move { pool.get().await }.map_ok(move |client| (height, client))
                    },
                ),
            )
        }
        .fuse();

        let mut sync_blocks = FuturesOrdered::new();
        let mut pending_blocks = FuturesOrdered::new();

        let mut new_blocks: futures::stream::Fuse<NewBlocksStream> =
            subscribe_new_blocks(self.pool.clone())
                .await
                .wrap_err("failed to start initial new-blocks subscription")?
                .fuse();

        let mut resubscribe = futures::future::Fuse::terminated();
        loop {
            select! {
                shutdown = &mut self.shutdown => {
                    match shutdown {
                        Err(e) => warn!(error.message = %e, "shutdown channel return with error; shutting down"),
                        Ok(()) => info!("received shutdown signal; shutting down"),
                    }
                    break;
                }

                // Drain the stream of heights to sync the sequencer reader.
                // The stream is fused, so after it exhausted this match arm will be deactivated.
                // The condition on the length of priority blocks relies on the pool being currently set
                // to 50. This ensures that no more than 20 requests to the sequencer are active at the same time.
                // Leaving some objects in the pool is important, so that the match arm for resolving priority blocks
                // below can reschedule blocks.
                Some(res) = sync_heights.next(), if !sync_heights.is_done() && sync_blocks.len() < 20 => {
                    match res {
                        Ok((height, client)) => sync_blocks.push_back(
                            async move { client.sequencer_block(height).await }
                            .map(move |res| (height, res))
                            .boxed()
                        ),
                        Err(e) => {
                            warn!(error.message = %e, error.cause = ?e, "sync stream failed; exiting reader");
                            break;
                        }
                    }
                }

                // Forward sync'ed blocks to the executor in the order of their heights. If an error with the
                // underlying transport was encountered, a new future is pushed to the front of the ordered
                // stream so that it is resolved first.
                // TODO: Are there conditions under which we repeatedly encounter jsonrpc errors, making this
                // an infinite process? At which point should the height just be dropped?
                Some((height, res)) = sync_blocks.next(), if !sync_blocks.is_empty() => {
                    match res {
                        Err(e) if e.as_tendermint_rpc().is_some() => {
                            let pool = self.pool.clone();
                            sync_blocks.push_front(
                                async move { let client = pool.get().await.unwrap(); client.sequencer_block(height).await }
                                .map(move |res| (height, res))
                                .boxed()
                            )
                        }
                        Err(e) => {
                            warn!(height, error.message = %e, error.cause = ?e, "getting sequencer block for given height failed; dropping it");
                        }
                        Ok(block) => self.forward_block(block),
                    }
                }

                // New blocks from the subscription to the sequencer. If this fused stream ever returns `None`,
                // a task is scheduled to resubscribe to new blocks.
                // New blocks are pushed into `pending_blocks` so that they are forwarded to the executor in the
                // order they were received.
                new_block = new_blocks.next(), if !new_blocks.is_done() => {
                    if let Some(block) = new_block {
                        pending_blocks.push_back(futures::future::ready(block));
                    } else {
                        warn!("sequencer new-block subscription closed unexpectedly; attempting to resubscribe");
                        let pool = self.pool.clone();
                        resubscribe = tokio::spawn(subscribe_new_blocks(pool)).fuse();
                    }
                }

                // Regular pending blocks will be submitted to the executor in the order they were received.
                // The condition on priority_blocks ensures that blocks from the sync process are sent first.
                Some(res) = pending_blocks.next(), if sync_blocks.is_empty() && !pending_blocks.is_empty() => {
                    match res {
                        Err(e) => {
                            warn!(error.message = %e, error.cause = ?e, "response from sequencer block subscription was bad; dropping it");
                        }
                        Ok(block) => {
                            self.forward_block(block);
                        }
                    };
                }

                res = &mut resubscribe => {
                    match res {
                        Ok(Ok::<NewBlocksStream, _>(subscription)) => {
                            new_blocks = subscription.fuse();
                        }
                        Ok(Err(e)) => {
                            warn!(error.message = %e, error.cause = ?e, "failed subscribing to new blocks from sequencer; exiting reader");
                            break;
                        }
                        Err(e) => {
                            warn!(error.message = %e, error.cause = ?e, "task attempting to subscribe to new blocks from sequencer failed; exiting reader");
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn forward_block(&self, block: SequencerBlockData) {
        if let Err(err) = self
            .executor_tx
            .send(ExecutorCommand::BlockReceivedFromSequencer {
                block: Box::new(block),
            })
        {
            warn!(err.msg = %err, err.cause = ?err, "failed sending new block received from sequencer to executor");
        }
    }
}

async fn subscribe_new_blocks(
    pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
) -> eyre::Result<NewBlocksStream> {
    use sequencer_client::SequencerSubscriptionClientExt as _;
    pool.get()
        .await
        .wrap_err("failed getting a sequencer client from the pool")?
        .subscribe_new_block_data()
        .await
        .wrap_err("failed subscribing to sequencer to receive new blocks")
}
