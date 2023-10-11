//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

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
use deadpool::managed::Pool;
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
};
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::{
    client_provider::ClientProvider,
    executor,
    executor::ExecutorCommand,
};

mod sync;

type ResubFut =
    future::Fuse<Pin<Box<dyn Future<Output = eyre::Result<NewBlocksStream>> + Send + 'static>>>;

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The start height from which to start syncing sequencer blocks.
    start_sync_height: u32,

    /// The object pool providing clients to the sequencer.
    pool: Pool<ClientProvider>,

    /// The shutdown channel to notify `Reader` to shut down.
    shutdown: oneshot::Receiver<()>,

    /// The sync-done channel to notify `Conductor` that `Reader` has finished syncing.
    sync_done: oneshot::Sender<()>,
}

impl Reader {
    pub(crate) fn new(
        start_sync_height: u32,
        pool: Pool<ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
        sync_done: oneshot::Sender<()>,
    ) -> Self {
        Self {
            start_sync_height,
            executor_tx,
            pool,
            shutdown,
            sync_done,
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use futures::{
            future::FusedFuture as _,
            stream::FuturesOrdered,
            FutureExt as _,
            StreamExt as _,
        };
        let Self {
            executor_tx,
            start_sync_height,
            pool,
            mut shutdown,
            sync_done,
        } = self;

        let mut new_blocks: stream::Fuse<NewBlocksStream> = subscribe_new_blocks(pool.clone())
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
            height.initial = start_sync_height,
            height.latest = latest_height,
            "syncing sequencer between configured initial and latest retrieved height"
        );

        let mut sync = sync::run(
            start_sync_height,
            latest_height,
            pool.clone(),
            executor_tx.clone(),
        )
        .boxed()
        .fuse();

        let mut sync_done = Some(sync_done);
        let mut resubscribe = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                shutdown = &mut shutdown => {
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

                res = &mut sync, if !sync.is_terminated() => {
                    if let Err(e) = res {
                        warn!(error.message = %e, error.cause = ?e, "sync failed; continuing with normal operation");
                    } else {
                        info!("sync finished successfully");
                    }
                    let sync_done = sync_done.take().expect("channel should only be used once and only in this branch; this is a bug");
                    let _ = sync_done.send(());
                }

                // New blocks from the subscription to the sequencer. If this fused stream ever returns `None`,
                // a task is scheduled to resubscribe to new blocks.
                // Blocks are pushed into `pending_blocks` so that they are forwarded to the executor in the
                // order they were received.
                new_block = new_blocks.next(), if !new_blocks.is_done() => {
                    schedule_for_forwarding_or_resubscribe(
                        &mut resubscribe,
                        &mut pending_blocks,
                        pool.clone(),
                        new_block,
                    );
                }

                // Regular pending blocks will be submitted to the executor in the order they were received.
                // The condition on `sync` ensures that blocks from the sync process are forwarded first.
                Some(res) = pending_blocks.next(), if sync.is_terminated() => {
                    if let Err(e) = forward_pending_block(executor_tx.clone(), res) {
                        error!("failed forwarding blocks during regular operation; exiting reader");
                        break 'reader_loop Err(e).wrap_err("failed forwarding blocks regular operation");
                    }
                }

                res = &mut resubscribe, if !resubscribe.is_terminated() => {
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
    res: eyre::Result<NewBlocksStream>,
) -> eyre::Result<()> {
    use futures::stream::StreamExt as _;

    let new_subscription = res.wrap_err("failed subscribing to new blocks from sequencer")?;
    *subscription = new_subscription.fuse();

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
        .send(ExecutorCommand::FromSequencer {
            block: Box::new(block),
        })
        .wrap_err("failed sending new block received from sequencer to executor")
}

fn schedule_for_forwarding_or_resubscribe(
    resubscribe: &mut ResubFut,
    pending_blocks: &mut FuturesOrdered<Ready<Result<SequencerBlockData, NewBlockStreamError>>>,
    pool: Pool<ClientProvider>,
    res: Option<Result<SequencerBlockData, NewBlockStreamError>>,
) {
    use futures::future::FutureExt as _;
    if let Some(res) = res {
        pending_blocks.push_back(futures::future::ready(res));
    } else {
        warn!("sequencer new-block subscription closed unexpectedly; attempting to resubscribe");
        *resubscribe = subscribe_new_blocks(pool).boxed().fuse();
    }
}
