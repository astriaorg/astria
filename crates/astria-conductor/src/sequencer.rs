//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.
use std::{
    future::Future,
    pin::Pin,
};

use astria_sequencer_types::SequencerBlockData;
use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use deadpool::managed::{
    Object,
    Pool,
    PoolError,
};
use futures::{
    future::{
        self,
        Ready,
    },
    stream::{
        self,
        FuturesOrdered,
    },
};
use sequencer_client::{
    extension_trait::NewBlocksStream,
    NewBlockStreamError,
};
use tokio::{
    select,
    sync::oneshot,
    task::{
        JoinError,
        JoinHandle,
    },
};
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::{
    client_provider::{
        self,
        ClientProvider,
    },
    executor,
    executor::ExecutorCommand,
};

type SyncBlockStream = FuturesOrdered<Pin<Box<dyn Future<Output = (u32, SyncBlockResult)> + Send>>>;

type SyncBlockResult = Result<SequencerBlockData, Error>;

type SyncHeightResult = Result<(u32, Object<ClientProvider>), PoolError<client_provider::Error>>;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed requesting a client from the pool")]
    Pool(#[from] PoolError<client_provider::Error>),
    #[error("sequencer request failed")]
    Request(#[from] sequencer_client::extension_trait::Error),
}

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    initial_sequencer_block_height: u32,

    pool: Pool<ClientProvider>,

    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    pub(crate) fn new(
        initial_sequencer_block_height: u32,
        pool: Pool<ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
    ) -> Self {
        Self {
            initial_sequencer_block_height,
            executor_tx,
            pool,
            shutdown,
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        use futures::{
            stream::FuturesOrdered,
            StreamExt as _,
            TryFutureExt as _,
        };
        let mut new_blocks: stream::Fuse<NewBlocksStream> = subscribe_new_blocks(self.pool.clone())
            .await
            .wrap_err("failed to start initial new-blocks subscription")?
            .fuse();

        let mut pending_blocks = FuturesOrdered::new();
        let latest_height = match new_blocks.next().await {
            None => bail!("subscription to sequencer for new blocks failed immediately; bailing"),
            Some(Err(e)) => {
                return Err(e).wrap_err("first sequencer block returned from subscription was bad");
            }
            Some(Ok(block)) => {
                let height = block.header().height.value();
                pending_blocks.push_back(futures::future::ready(Ok(block)));
                height
            }
        };

        let latest_height: u32 = latest_height.try_into().wrap_err(
            "failed converting the cometbft height to u32, but this should always work",
        )?;

        info!(
            height.initial = self.initial_sequencer_block_height,
            height.latest = latest_height,
            "syncing sequencer between configured initial and latest retrieved height"
        );
        let mut sync_heights = {
            let sync_range = self.initial_sequencer_block_height..latest_height;
            let pool = self.pool.clone();
            Box::pin(futures::stream::iter(sync_range).then(move |height| {
                let pool = pool.clone();
                async move { pool.get().await }.map_ok(move |client| (height, client))
            }))
        }
        .fuse();

        let mut sync_blocks = FuturesOrdered::new();

        let mut resubscribe = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                shutdown = &mut self.shutdown => {
                    let ret = match shutdown {
                        Err(e) => {
                            warn!(error.message = %e, "shutdown channel closed unexpectedly; shutting down");
                            Err(e).wrap_err("shut down channel closed unexpectedly")
                        }
                        Ok(()) => {
                            info!("received shutdown signal; shutting down");
                            Ok(())
                        }
                    };
                    break 'reader_loop ret;
                }

                // Drain the stream of heights to sync the sequencer reader.
                // The stream is fused, so after it exhausted this match arm will be deactivated.
                // The condition on the length of priority blocks relies on the pool being currently set
                // to 50. This ensures that no more than 20 requests to the sequencer are active at the same time.
                // Leaving some objects in the pool is important, so that the match arm for resolving priority blocks
                // below can reschedule blocks.
                Some(res) = sync_heights.next(), if !sync_heights.is_done() && sync_blocks.len() < 20 => {
                    if let Err(e) = sync_block_at_height(&mut sync_blocks, res) {
                        error!(error.message = %e, error.cause = ?e, "syncing heights failed; exiting reader");
                        break 'reader_loop Err(e).wrap_err("syncing heights failed");
                    }
                }

                // Forward sync'ed blocks to the executor in the order of their heights. If an error with the
                // underlying transport was encountered, a new future is pushed to the front of the ordered
                // stream so that it is resolved first.
                // TODO: Are there conditions under which we repeatedly encounter jsonrpc errors, making this
                // an infinite process? At which point should the height just be dropped?
                Some((height, res)) = sync_blocks.next(), if !sync_blocks.is_empty() => {
                    if let Err(e) = forward_sync_block_or_reschedule(
                        &mut sync_blocks,
                        self.pool.clone(),
                        self.executor_tx.clone(),
                        height,
                        res,
                    ) {
                        error!("failed forwarding blocks during sync; exiting reader");
                        break 'reader_loop Err(e).wrap_err("failed forwarding blocks during sync");
                    }
                }

                // New blocks from the subscription to the sequencer. If this fused stream ever returns `None`,
                // a task is scheduled to resubscribe to new blocks.
                // New blocks are pushed into `pending_blocks` so that they are forwarded to the executor in the
                // order they were received.
                new_block = new_blocks.next(), if !new_blocks.is_done() => {
                    schedule_for_forwarding_or_resubscribe(
                        &mut resubscribe,
                        &mut pending_blocks,
                        self.pool.clone(),
                        new_block,
                    );
                }

                // Regular pending blocks will be submitted to the executor in the order they were received.
                // The condition on priority_blocks ensures that blocks from the sync process are sent first.
                Some(res) = pending_blocks.next(), if sync_blocks.is_empty() && !pending_blocks.is_empty() => {
                    if let Err(e) = forward_pending_block(self.executor_tx.clone(), res) {
                        error!("failed forwarding blocks during regular operation; exiting reader");
                        break 'reader_loop Err(e).wrap_err("failed forwarding blocks regular operation");
                    }
                }

                res = &mut resubscribe => {
                    if let Err(e) = assign_new_blocks_subscription(&mut new_blocks, res) {
                        error!(error.message = %e, error.cause = ?e, "failed to resubscribe to get new blocks from sequencer; exiting reader");
                        break 'reader_loop Err(e).wrap_err("failed to resubscribe to new blocks from sequencer");
                    }
                }
            }
        }
    }
}

fn assign_new_blocks_subscription(
    subscription: &mut stream::Fuse<NewBlocksStream>,
    res: Result<eyre::Result<NewBlocksStream>, JoinError>,
) -> eyre::Result<()> {
    use futures::stream::StreamExt as _;

    let new_subscription = res
        .wrap_err("task to subscribe to new blocks from sequencer was aborted prematurely")?
        .wrap_err("failed subscribing to new blocks from sequencer")?;
    *subscription = new_subscription.fuse();

    Ok(())
}

fn sync_block_at_height(
    sync_stream: &mut SyncBlockStream,
    res: SyncHeightResult,
) -> eyre::Result<()> {
    use futures::future::FutureExt as _;
    use sequencer_client::SequencerClientExt as _;
    let (height, client) = res.wrap_err("failed requesting a client from the pool")?;
    sync_stream.push_back(
        async move { client.sequencer_block(height).await.map_err(Error::Request) }
            .map(move |res| (height, res))
            .boxed(),
    );
    Ok(())
}

async fn subscribe_new_blocks(pool: Pool<ClientProvider>) -> eyre::Result<NewBlocksStream> {
    use sequencer_client::SequencerSubscriptionClientExt as _;
    pool.get()
        .await
        .wrap_err("failed getting a sequencer client from the pool")?
        .subscribe_new_block_data()
        .await
        .wrap_err("failed subscribing to sequencer to receive new blocks")
}

fn forward_sync_block_or_reschedule(
    sync_stream: &mut SyncBlockStream,
    pool: Pool<ClientProvider>,
    executor_tx: executor::Sender,
    height: u32,
    res: SyncBlockResult,
) -> eyre::Result<()> {
    use futures::future::FutureExt as _;
    use sequencer_client::SequencerClientExt as _;

    let block = match res {
        Err(e) => {
            let mut rescheduled = false;
            if let Error::Request(e) = &e {
                if let Some(err) = e.as_tendermint_rpc() {
                    if err.is_transport() {
                        sync_stream.push_front(
                            async move {
                                let client = pool.get().await?;
                                let block = client.sequencer_block(height).await?;
                                Ok(block)
                            }
                            .map(move |res| (height, res))
                            .boxed(),
                        );
                        rescheduled = true;
                    }
                }
            }
            if rescheduled {
                warn!(error.message = %e, error.cause = ?e, "rescheduling fetch of sequencer block because underlying transport failed");
            } else {
                warn!(error.message = %e, error.cause = ?e, "failed syncing block; dropping the height from sync");
            }
            return Ok(());
        }
        Ok(block) => block,
    };
    executor_tx
        .send(ExecutorCommand::BlockReceivedFromSequencer {
            block: Box::new(block),
        })
        .wrap_err("failed sending new block received from sequencer to executor")
}

fn forward_pending_block(
    executor_tx: executor::Sender,
    res: Result<SequencerBlockData, NewBlockStreamError>,
) -> eyre::Result<()> {
    let block = match res {
        Err(e) => {
            warn!(error.message = %e, error.cause = ?e, "response from sequencer block subscription was bad; dropping it");
            return Ok(());
        }
        Ok(block) => block,
    };
    executor_tx
        .send(ExecutorCommand::BlockReceivedFromSequencer {
            block: Box::new(block),
        })
        .wrap_err("failed sending new block received from sequencer to executor")
}

fn schedule_for_forwarding_or_resubscribe(
    resubscribe: &mut future::Fuse<JoinHandle<eyre::Result<NewBlocksStream>>>,
    pending_blocks: &mut FuturesOrdered<Ready<Result<SequencerBlockData, NewBlockStreamError>>>,
    pool: Pool<ClientProvider>,
    res: Option<Result<SequencerBlockData, NewBlockStreamError>>,
) {
    use futures::future::FutureExt as _;
    if let Some(res) = res {
        pending_blocks.push_back(futures::future::ready(res));
    } else {
        warn!("sequencer new-block subscription closed unexpectedly; attempting to resubscribe");
        *resubscribe = tokio::spawn(subscribe_new_blocks(pool)).fuse();
    }
}
