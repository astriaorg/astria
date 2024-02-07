//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

use std::{
    future::Future,
    pin::Pin,
};

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
    tendermint::block::Height,
    NewBlockStreamError,
    SequencerBlock,
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
};

mod sync;

type ResubFut = future::Fuse<
    Pin<Box<dyn Future<Output = Result<NewBlocksStream, ResubscriptionError>> + Send + 'static>>,
>;

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The object pool providing clients to the sequencer.
    pool: Pool<ClientProvider>,

    /// The shutdown channel to notify `Reader` to shut down.
    shutdown: oneshot::Receiver<()>,

    /// The start height from which to start syncing sequencer blocks.
    start_sync_height: Height,
}

impl Reader {
    pub(crate) fn new(
        start_sync_height: Height,
        pool: Pool<ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
    ) -> Self {
        Self {
            executor_tx,
            pool,
            shutdown,
            start_sync_height,
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
                let height = block.header().height;
                pending_blocks.push_back(futures::future::ready(block));
                height
            }
        };

        let mut next_height = latest_height;

        info!(
            height.initial = %start_sync_height,
            height.latest = %next_height,
            "syncing sequencer between configured initial and latest retrieved height"
        );

        let mut sync = sync::run(
            start_sync_height,
            next_height,
            pool.clone(),
            executor_tx.clone(),
        )
        .boxed()
        .fuse();

        let mut resubscribe = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                shutdown = &mut shutdown => {
                    let ret = if let Err(e) = shutdown {
                        let error = &e as &(dyn std::error::Error + 'static);
                        warn!(error, "shutdown channel closed unexpectedly; shutting down");
                        Err(e).wrap_err("shut down channel closed unexpectedly")
                    } else {
                        info!("received shutdown signal; shutting down");
                        Ok(())
                    };
                    break 'reader_loop ret;
                }

                res = &mut sync, if !sync.is_terminated() => {
                    if let Err(e) = res {
                        let error: &(dyn std::error::Error + 'static) = e.as_ref();
                        warn!(error, "sync failed; continuing with normal operation");
                    } else {
                        info!("sync finished successfully");
                    }
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
                Some(block) = pending_blocks.next(), if sync.is_terminated() => {
                    match forward_block_or_resync(
                        block,
                        next_height,
                        &mut pending_blocks,
                        &mut sync,
                        pool.clone(),
                        executor_tx.clone())
                    {
                        Err(e) => {
                            let error: &(dyn std::error::Error + 'static) = e.as_ref();
                            error!(error, "fatally failed to handle new pending block; exiting reader loop");
                            break 'reader_loop Err(e);
                        }
                        Ok(new_next_height) => next_height = new_next_height,
                    }
                }

                res = &mut resubscribe, if !resubscribe.is_terminated() => {
                    match res {
                        Ok(new_subscription) => {
                          new_blocks = new_subscription.fuse();
                        }
                        Err(err) => {
                            let error: &(dyn std::error::Error + 'static) = &err;
                            warn!(error, "failed resubscribing to new blocks stream; trying again");
                            let pool = pool.clone();
                            resubscribe = subscribe_new_blocks(pool).boxed().fuse();
                        }
                    }
                }
            };
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ResubscriptionError {
    #[error("failed getting a sequencer client from the pool")]
    Pool(#[from] deadpool::managed::PoolError<crate::client_provider::Error>),
    #[error("JSONRPC to subscribe to new blocks failed")]
    JsonRpc(#[from] sequencer_client::extension_trait::SubscriptionFailed),
    #[error("back off failed after 1024 attempts")]
    BackoffFailed,
}

impl Clone for ResubscriptionError {
    fn clone(&self) -> Self {
        use deadpool::managed::{
            HookError,
            PoolError,
        };
        match self {
            Self::Pool(e) => {
                let e = match e {
                    PoolError::Timeout(t) => PoolError::Timeout(*t),
                    PoolError::Backend(e) => PoolError::Backend(e.clone()),
                    PoolError::Closed => PoolError::Closed,
                    PoolError::NoRuntimeSpecified => PoolError::NoRuntimeSpecified,
                    PoolError::PostCreateHook(HookError::Message(s)) => {
                        PoolError::PostCreateHook(HookError::Message(s.clone()))
                    }
                    PoolError::PostCreateHook(HookError::StaticMessage(m)) => {
                        PoolError::PostCreateHook(HookError::StaticMessage(m))
                    }
                    PoolError::PostCreateHook(HookError::Backend(e)) => {
                        PoolError::PostCreateHook(HookError::Backend(e.clone()))
                    }
                };
                Self::Pool(e)
            }
            Self::JsonRpc(e) => ResubscriptionError::JsonRpc(e.clone()),
            Self::BackoffFailed => ResubscriptionError::BackoffFailed,
        }
    }
}

async fn subscribe_new_blocks(
    pool: Pool<ClientProvider>,
) -> Result<NewBlocksStream, ResubscriptionError> {
    use std::time::Duration;

    use sequencer_client::SequencerSubscriptionClientExt as _;
    let retry_config = tryhard::RetryFutureConfig::new(10)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(10))
        .on_retry(
            |attempt, next_delay: Option<std::time::Duration>, error: &ResubscriptionError| {
                let error = error.clone();
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                async move {
                    let error: &(dyn std::error::Error + 'static) = &error;
                    warn!(
                        attempt,
                        wait_duration,
                        error,
                        "attempt to resubscribe to new blocks from sequencer failed; retrying \
                         after backoff",
                    );
                }
            },
        );

    tryhard::retry_fn(move || {
        let pool = pool.clone();
        async move {
            let subscription = pool.get().await?.subscribe_new_block_data().await?;
            Ok(subscription)
        }
    })
    .with_config(retry_config)
    .await
    .map_err(|_| ResubscriptionError::BackoffFailed)
}

/// Forwards a sequencer block to the executor if it contains the expected height, or reschedules
/// the block for later while fetching blocks for all missing heights.
///
/// The following cases are considered (with `h` the next height expected by the sequencer reader,
/// and `k` the height recorded in the block):
///
/// 1. if `h == k` the block is forwarded to the executor. `h+1` is returned as the next expected
///    height.
/// 2. if `h < k` the block is dropped. `h` is returned as the next expected height (i.e. there is
///    no change in the expected height).
/// 3. if `h > k` a re-sync is scheduled for the range `h..k` (exluding `k`), the block is pushed to
///    the front of the queue to be forwarded later. `k` (the height of the re-scheduled block) is
///    returned as the next expected height.
///
/// # Returns
/// Returns the next expected height, depending on the cases discussed above.
///
/// # Errors
/// Returns an error if a block could not be sent to the executor. This is fatal
/// and can only happen if the executor is shut down.
// TODO: bring back instrument
// #[instrument(
//     skip_all,
//     fields(
//         height.expected = %expected_height,
//         height.block = %block.header().height,
//         block.hash = %block.block_hash()
//     )
// )]
fn forward_block_or_resync(
    block: SequencerBlock,
    expected_height: Height,
    pending_blocks: &mut FuturesOrdered<Ready<SequencerBlock>>,
    sync: &mut future::Fuse<Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>>,
    pool: Pool<ClientProvider>,
    executor_tx: executor::Sender,
) -> eyre::Result<Height> {
    use futures::FutureExt as _;

    let block_height = block.header().height;

    match expected_height.cmp(&block_height) {
        // received block is at expected height: send to the executor
        std::cmp::Ordering::Equal => {
            executor_tx
                .send(block.into())
                .wrap_err("forwarding sequencer block to executor failed")?;
            Ok(expected_height.increment())
        }
        // received block is above expected height: start a re-sync and reschedule the block at its
        // height
        std::cmp::Ordering::Less => {
            pending_blocks.push_front(future::ready(block));
            let missing_start = expected_height;
            let missing_end = block_height;
            *sync = sync::run(missing_start, missing_end, pool, executor_tx)
                .boxed()
                .fuse();
            Ok(block_height)
        }
        // received block is below expected height: drop it
        std::cmp::Ordering::Greater => Ok(expected_height),
    }
}

fn schedule_for_forwarding_or_resubscribe(
    resubscribe: &mut ResubFut,
    pending_blocks: &mut FuturesOrdered<Ready<SequencerBlock>>,
    pool: Pool<ClientProvider>,
    res: Option<Result<SequencerBlock, NewBlockStreamError>>,
) {
    use futures::future::FutureExt as _;
    if let Some(res) = res {
        match res {
            Err(e) => {
                let error = &e as &(dyn std::error::Error + 'static);
                warn!(
                    error,
                    "block received from sequencer subscription was bad; dropping it"
                );
            }
            Ok(block) => pending_blocks.push_back(futures::future::ready(block)),
        }
    } else {
        warn!("sequencer new-block subscription closed unexpectedly; attempting to resubscribe");
        *resubscribe = subscribe_new_blocks(pool).boxed().fuse();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
    };

    use astria_core::sequencer::v1alpha1::test_utils::make_cometbft_block;
    use color_eyre::eyre;
    use futures::{
        future::{
            self,
            Fuse,
            FusedFuture as _,
            Ready,
        },
        stream::FuturesOrdered,
    };

    use super::{
        forward_block_or_resync,
        SequencerBlock,
    };
    use crate::{
        client_provider::mock::TestPool,
        executor::ExecutorCommand,
    };

    struct ForwardBlockOrResyncEnvironment {
        pending_blocks: FuturesOrdered<Ready<SequencerBlock>>,
        sync: future::Fuse<Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>>,
        test_pool: TestPool,
        executor_rx: crate::executor::Receiver,
        executor_tx: crate::executor::Sender,
    }

    impl ForwardBlockOrResyncEnvironment {
        async fn setup() -> Self {
            let pending_blocks = FuturesOrdered::new();
            let sync = Fuse::terminated();
            let test_pool = TestPool::setup().await;
            let (executor_tx, executor_rx) = tokio::sync::mpsc::unbounded_channel();
            Self {
                pending_blocks,
                sync,
                test_pool,
                executor_rx,
                executor_tx,
            }
        }
    }

    #[tokio::test]
    async fn block_at_expected_height_is_forwarded() {
        let expected_height = 5u32.into();
        let mut block = make_cometbft_block();
        block.header.height = expected_height;
        let expected_block = SequencerBlock::try_from_cometbft(block)
            .expect("the tendermint block should be well formed");

        let mut env = ForwardBlockOrResyncEnvironment::setup().await;
        let next_height = forward_block_or_resync(
            expected_block.clone(),
            expected_height,
            &mut env.pending_blocks,
            &mut env.sync,
            env.test_pool.pool.clone(),
            env.executor_tx,
        )
        .expect("the receiver is alive");
        assert!(
            env.pending_blocks.is_empty(),
            "block should not be rescheduled"
        );
        assert!(env.sync.is_terminated(), "resync should not be triggered");
        let ExecutorCommand::FromSequencer {
            block: actual_block,
        } = env
            .executor_rx
            .try_recv()
            .expect("block should be forwarded")
        else {
            panic!("value sent to executor should be a ExecutorCommand::FromSequencer variant");
        };
        assert_eq!(expected_block, *actual_block, "block should not change");
        assert_eq!(
            expected_height.increment(),
            next_height,
            "next height should be previous height + 1"
        );
    }

    #[tokio::test]
    async fn future_block_triggers_resync() {
        let expected_height = 5u32.into();
        let future_height = 8u32.into();
        let mut block = make_cometbft_block();
        block.header.height = future_height;
        let expected_block = SequencerBlock::try_from_cometbft(block)
            .expect("the tendermint block should be well formed");

        let mut env = ForwardBlockOrResyncEnvironment::setup().await;
        let next_height = forward_block_or_resync(
            expected_block.clone(),
            expected_height,
            &mut env.pending_blocks,
            &mut env.sync,
            env.test_pool.pool.clone(),
            env.executor_tx,
        )
        .expect("the receiver is alive");
        assert_eq!(1, env.pending_blocks.len(), "block should be rescheduled");
        assert!(!env.sync.is_terminated(), "sync should be triggered");
        env.executor_rx
            .try_recv()
            .expect_err("block should be rescheduled, not fowarded");
        assert_eq!(
            future_height, next_height,
            "next height should be that of the future block"
        );
    }

    #[tokio::test]
    async fn older_block_is_dropped() {
        let expected_height = 5u32.into();
        let mut block = make_cometbft_block();
        block.header.height = 3u32.into();
        let expected_block = SequencerBlock::try_from_cometbft(block)
            .expect("the tendermint block should be well formed");

        let mut env = ForwardBlockOrResyncEnvironment::setup().await;
        let next_height = forward_block_or_resync(
            expected_block.clone(),
            expected_height,
            &mut env.pending_blocks,
            &mut env.sync,
            env.test_pool.pool.clone(),
            env.executor_tx,
        )
        .expect("the receiver is alive");
        assert!(
            env.pending_blocks.is_empty(),
            "block should not be rescheduled"
        );
        assert!(env.sync.is_terminated(), "resync should not be triggered");
        env.executor_rx
            .try_recv()
            .expect_err("block should be dropped, not fowarded");
        assert_eq!(
            expected_height, next_height,
            "next height should be the same expected height",
        );
    }
}
