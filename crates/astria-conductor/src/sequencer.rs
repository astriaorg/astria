//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

use std::{
    collections::VecDeque,
    error::Error as StdError,
    pin::Pin,
    task::Poll,
};

use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use deadpool::managed::{
    Pool,
    PoolError,
};
use futures::{
    future::{
        self,
        BoxFuture,
    },
    stream::FuturesUnordered,
    FutureExt as _,
    Stream,
    StreamExt as _,
};
use pin_project_lite::pin_project;
use sequencer_client::{
    extension_trait::NewBlocksStream,
    tendermint::block::Height,
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
    block_cache::BlockCache,
    client_provider::{
        self,
        ClientProvider,
    },
    executor,
};

pub(crate) struct Reader {
    executor: executor::Handle,

    /// The object pool providing clients to the sequencer.
    pool: Pool<ClientProvider>,

    /// The shutdown channel to notify `Reader` to shut down.
    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    pub(crate) fn new(
        pool: Pool<ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor: executor::Handle,
    ) -> Self {
        Self {
            executor,
            pool,
            shutdown,
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use futures::future::FusedFuture as _;
        let Self {
            mut executor,
            pool,
            mut shutdown,
        } = self;

        let mut executor = executor
            .wait_for_init()
            .await
            .wrap_err("handle to executor failed while waiting for it being initialized")?;
        let start_height = executor.next_expected_soft_height();

        let mut subscription = resubscribe(pool.clone())
            .await
            .wrap_err("failed to start initial new-blocks subscription")?
            .fuse();

        let mut sequential_blocks = BlockCache::with_next_height(start_height);
        let latest_height = match subscription.next().await {
            None => bail!("subscription to sequencer for new blocks failed immediately; bailing"),
            Some(Err(e)) => {
                return Err(e).wrap_err("first sequencer block returned from subscription was bad");
            }
            Some(Ok(block)) => {
                let height = block.header().height;
                if let Err(e) = sequential_blocks.insert(block) {
                    warn!(
                        error = &e as &dyn StdError,
                        "latest sequencer block couldn't be inserted into block cache; is the \
                         start sync height in the future?",
                    );
                }
                height
            }
        };

        let mut blocks_from_height =
            BlocksFromHeightStream::new(start_height, latest_height, pool.clone(), 20);

        let mut resubscribing = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                shutdown = &mut shutdown => {
                    let ret = if let Err(e) = shutdown {
                        warn!(
                            error = &e as &dyn StdError,
                            "shutdown channel closed unexpectedly; shutting down",
                        );
                        Err(e).wrap_err("shut down channel closed unexpectedly")
                    } else {
                        info!("received shutdown signal; shutting down");
                        Ok(())
                    };
                    break 'reader_loop ret;
                }

                Some(block) = blocks_from_height.next() => {
                    if let Err(e) = sequential_blocks.insert(block) {
                        // XXX: we could temporarily kill the subscription if we put an upper limit on the cache size
                        warn!(error = &e as &dyn std::error::Error, "failed pushing block into cache, dropping it");
                    }
                }

                Ok(next_height) = executor.next_expected_soft_height_if_changed() => {
                    sequential_blocks.drop_obsolete(next_height);
                    blocks_from_height.skip_to_height(next_height);
                }

                Some(block) = sequential_blocks.next_block() => {
                    if let Err(e) = executor.send_soft(block) {
                        let reason = "failed sending next sequencer block to executor";
                        error!(
                            error = &e as &dyn std::error::Error,
                            reason,
                            "exiting",
                        );
                        break 'reader_loop Err(e).wrap_err(reason);
                    }
                }

                // Blocks from the sequencer subscription. Resubscribes if `None`
                new_block = subscription.next(), if !subscription.is_done() => {
                    match new_block {
                        Some(Ok(block)) => {
                            let height = block.height();
                            if let Err(e) = sequential_blocks.insert(block) {
                                // XXX: we could temporarily kill the subscription if we put an upper limit on the cache size
                                warn!(error = &e as &dyn std::error::Error, "failed pushing block into cache, dropping it");
                            } else {
                                blocks_from_height.advance_height(height);
                            }
                        },
                        Some(Err(e)) => warn!(
                            error = &e as &dyn StdError,
                            "received bad block from sequencer subscription; dropping it"
                        ),
                        None => {
                            warn!("sequencer new-block subscription terminated unexpectedly; attempting to resubscribe");
                            resubscribing = resubscribe(pool.clone());
                        }
                    }
                }

                new_subscription = &mut resubscribing, if !resubscribing.is_terminated() => {
                    match new_subscription {
                        Ok(new_subscription) => {
                          subscription = new_subscription.fuse();
                        }
                        Err(e) => {
                            warn!(error = &e as &dyn StdError, "failed resubscribing to new blocks stream; trying again");
                            resubscribing = resubscribe(pool.clone());
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

fn resubscribe(
    pool: Pool<ClientProvider>,
) -> future::Fuse<BoxFuture<'static, Result<NewBlocksStream, ResubscriptionError>>> {
    use std::time::Duration;

    use futures::TryFutureExt as _;
    use sequencer_client::SequencerSubscriptionClientExt as _;
    let retry_config = tryhard::RetryFutureConfig::new(10)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(10))
        .on_retry(
            |attempt, next_delay: Option<std::time::Duration>, error: &ResubscriptionError| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn StdError,
                    "attempt to resubscribe to new blocks from sequencer failed; retrying after \
                     backoff",
                );
                std::future::ready(())
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
    .map_err(|_| ResubscriptionError::BackoffFailed)
    .boxed()
    .fuse()
}

pin_project! {
    struct BlocksFromHeightStream {
        heights: VecDeque<Height>,
        greatest_seen_height: Height,
        in_progress_queue: FuturesUnordered<BoxFuture<'static, Result<SequencerBlock, BlockFetchError>>>,
        pool: Pool<ClientProvider>,
        max: usize,
    }
}

impl BlocksFromHeightStream {
    // Registers a new block's height, advancing the streams greatest seen height and
    // pushing missing heights into its queue if necessary.
    fn advance_height(&mut self, height: Height) {
        loop {
            let next_height = height.increment();
            match next_height.cmp(&height) {
                std::cmp::Ordering::Less => {
                    self.heights.push_back(next_height);
                    self.greatest_seen_height = next_height;
                }

                std::cmp::Ordering::Equal => {
                    self.greatest_seen_height = next_height;
                    break;
                }

                std::cmp::Ordering::Greater => break,
            }
        }
    }

    /// Drops all heights lower than the given height.
    ///
    /// NOTE: This requires that `self.heights` are always ordered!
    fn skip_to_height(&mut self, height: Height) {
        let i = self.heights.partition_point(|&in_deque| in_deque < height);
        for _ in 0..i {
            self.heights.pop_front();
        }
    }

    fn new(start: Height, end: Height, pool: Pool<ClientProvider>, max: usize) -> Self {
        let heights: VecDeque<_> = crate::utils::height_range_exclusive(start, end).collect();
        let greatest_seen_height = heights
            .back()
            .copied()
            .expect("height range returns at least one element; this is a bug");
        Self {
            heights,
            greatest_seen_height,
            in_progress_queue: FuturesUnordered::new(),
            pool,
            max,
        }
    }
}

impl Stream for BlocksFromHeightStream {
    type Item = SequencerBlock;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        use futures::ready;
        let this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while this.in_progress_queue.len() < *this.max {
            match this.heights.pop_front() {
                Some(height) => this
                    .in_progress_queue
                    .push(fetch_client_then_block(this.pool.clone(), height).boxed()),
                None => break,
            }
        }

        // Attempt to pull the next value from the in_progress_queue
        let res = this.in_progress_queue.poll_next_unpin(cx);
        if let Some(val) = ready!(res) {
            match val {
                Ok(val) => return Poll::Ready(Some(val)),
                // XXX: Consider what to about the height here - push it back into the vecdeque?
                Err(e) => warn!(
                    error = &e as &dyn StdError,
                    "failed fetching celestia blobs for height, dropping height"
                ),
            }
        }

        // If more values are still coming from the stream, we're not done yet
        if this.heights.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = self.in_progress_queue.len();
        let n_heights = self.heights.len();
        let len = n_heights.saturating_add(queue_len);
        (len, Some(len))
    }
}

async fn fetch_client_then_block(
    pool: Pool<ClientProvider>,
    height: Height,
) -> Result<SequencerBlock, BlockFetchError> {
    use sequencer_client::SequencerClientExt as _;

    let client = pool.get().await?;
    let block = client.sequencer_block(height).await?;
    Ok(block)
}

#[derive(Debug, thiserror::Error)]
enum BlockFetchError {
    #[error("failed requesting a client from the pool")]
    Pool(#[from] PoolError<client_provider::Error>),
    #[error("getting a block from sequencer failed")]
    Request(#[from] sequencer_client::extension_trait::Error),
}
