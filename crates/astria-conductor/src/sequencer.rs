//! [`Reader`] reads reads blocks from sequencer and forwards them to [`crate::executor::Executor`].

use std::{
    error::Error as StdError,
    pin::Pin,
    task::Poll,
};

use deadpool::managed::{
    Pool,
    PoolError,
};
use eyre::{
    self,
    bail,
    WrapErr as _,
};
use futures::{
    future::{
        self,
        BoxFuture,
        Fuse,
    },
    FutureExt as _,
    Stream,
    StreamExt as _,
};
use futures_bounded::FuturesMap;
use pin_project_lite::pin_project;
use sequencer_client::{
    extension_trait::LatestHeightStream,
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
    trace,
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

    #[instrument(skip_all, err)]
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
        let next_expected_height = executor.next_expected_soft_height();

        let mut subscription = resubscribe(pool.clone())
            .await
            .wrap_err("failed to start initial new-blocks subscription")?
            .fuse();

        let latest_height = match subscription.next().await {
            None => bail!("subscription to sequencer for latest heights failed immediately"),
            Some(Err(e)) => {
                return Err(e).wrap_err("first latest height from sequencer was bad");
            }
            Some(Ok(height)) => height,
        };
        let mut sequential_blocks = BlockCache::with_next_height(next_expected_height)
            .wrap_err("failed constructing sequential block cache")?;
        let mut blocks_from_heights =
            BlocksFromHeightStream::new(next_expected_height, latest_height, pool.clone(), 20);

        let mut resubscribing = future::Fuse::terminated();
        let mut scheduled_send: Fuse<BoxFuture<Result<_, _>>> = future::Fuse::terminated();
        'reader_loop: loop {
            select! {
                shutdown = &mut shutdown => {
                    let ret = if let Err(e) = shutdown {
                        warn!(
                            error = &e as &dyn StdError,
                            "shutdown channel closed unexpectedly; shutting down",
                        );
                        Err(e).wrap_err("shutdown channel closed unexpectedly")
                    } else {
                        info!("received shutdown signal; shutting down");
                        Ok(())
                    };
                    break 'reader_loop ret;
                }

                Some(block) = blocks_from_heights.next() => {
                    let block = block.wrap_err("failed getting block")?;

                    if let Err(e) = sequential_blocks.insert(block) {
                        // XXX: we could temporarily kill the subscription if we put an upper limit on the cache size
                        warn!(error = &e as &dyn std::error::Error, "failed pushing block into cache, dropping it");
                    }
                }

                Ok(next_height) = executor.next_expected_soft_height_if_changed() => {
                    blocks_from_heights.record_next_expected_height(next_height);
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

                // Blocks from the sequencer subscription. Resubscribes if `None`
                latest_height = subscription.next(), if !subscription.is_done() => {
                    match latest_height {
                        Some(Ok(height)) => {
                            blocks_from_heights.record_latest_height(height);
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
            }
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
) -> future::Fuse<BoxFuture<'static, Result<LatestHeightStream, ResubscriptionError>>> {
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
                    "attempt to resubscribe to latest height from sequencer failed; retrying \
                     after backoff",
                );
                std::future::ready(())
            },
        );

    tryhard::retry_fn(move || {
        let pool = pool.clone();
        async move {
            let subscription = pool.get().await?.subscribe_latest_height().await?;
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
        next_expected_height: Height,
        greatest_requested_height: Option<Height>,
        latest_sequencer_height: Height,
        in_progress: FuturesMap<Height, Result<SequencerBlock, BlockFetchError>>,
        pool: Pool<ClientProvider>,
        max_ahead: u64,
    }
}

impl BlocksFromHeightStream {
    /// Records the latest height observed from sequencer.
    ///
    /// Ignores it if its older than what was previously observed.
    #[instrument(
        skip_all,
        fields(
            latest_height.observed = %height,
            latest_height.recorded = %self.latest_sequencer_height,
        )
    )]
    fn record_latest_height(&mut self, height: Height) {
        if height < self.latest_sequencer_height {
            info!("observed latest sequencer height older than previous; ignoring it",);
        }
        self.latest_sequencer_height = height;
    }

    /// Records the latest height observed from sequencer.
    ///
    /// Ignores it if its older than what was previously observed.
    #[instrument(
        skip_all,
        fields(
            next_height.observed = %height,
            next_height.recorded = %self.next_expected_height,
        )
    )]
    fn record_next_expected_height(&mut self, height: Height) {
        if height < self.next_expected_height {
            info!("next expected sequencer height older than previous; ignoring it",);
        }
        self.next_expected_height = height;
    }

    /// The stream can yield more if the greatest requested height isn't too far
    /// ahead of the next expected height and not ahead of the latest observed sequencer height.
    fn next_height_to_fetch(&self) -> Option<Height> {
        let potential_height = match self.greatest_requested_height {
            None => self.next_expected_height,
            Some(greatest_requested_height) => greatest_requested_height.increment(),
        };
        let not_too_far_ahead =
            potential_height.value() < (self.next_expected_height.value() + self.max_ahead);
        let height_exists_on_sequencer = potential_height <= self.latest_sequencer_height;
        if not_too_far_ahead && height_exists_on_sequencer {
            Some(potential_height)
        } else {
            None
        }
    }

    fn new(
        next_expected_height: Height,
        latest_sequencer_height: Height,
        pool: Pool<ClientProvider>,
        max_in_flight: usize,
    ) -> Self {
        Self {
            next_expected_height,
            latest_sequencer_height,
            greatest_requested_height: None,
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), max_in_flight),
            pool,
            max_ahead: 128,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed getting a block at height {height}")]
struct BlocksFromHeightStreamError {
    height: Height,
    source: BlockFetchError,
}

impl Stream for BlocksFromHeightStream {
    type Item = Result<SequencerBlock, BlocksFromHeightStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        use futures_bounded::PushError;

        // Try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Some(next_height) = self.as_ref().get_ref().next_height_to_fetch() {
            let this = self.as_mut().project();
            match this.in_progress.try_push(
                next_height,
                fetch_client_then_block(this.pool.clone(), next_height),
            ) {
                Err(PushError::BeyondCapacity(_)) => break,
                Err(PushError::Replaced(_)) => {
                    error!(
                        height = %next_height,
                        "scheduled to fetch block, but a fetch for the same height was already in-flight",
                    );
                }
                Ok(()) => {}
            }
            this.greatest_requested_height.replace(next_height);
        }

        // Attempt to pull the next value from the in_progress_queue
        let (height, res) = futures::ready!(self.as_mut().project().in_progress.poll_unpin(cx));

        // Ok branch (contains the block or a fetch error): propagate the error up
        //
        // Err branch (timeout): a fetch timing out is not a problem: we can just reschedule it.
        match res {
            Ok(fetch_result) => {
                return Poll::Ready(Some(fetch_result.map_err(|source| {
                    BlocksFromHeightStreamError {
                        height,
                        source,
                    }
                })));
            }
            Err(timed_out) => {
                warn!(
                    %height,
                    error = &timed_out as &dyn StdError,
                    "request for height timed out, rescheduling",
                );
                let res = {
                    let this = self.as_mut().project();
                    this.in_progress
                        .try_push(height, fetch_client_then_block(this.pool.clone(), height))
                };
                assert!(
                    res.is_ok(),
                    "there must be space in the map after a future timed out"
                );
            }
        }

        // We only reach this part if the `futures::ready!` didn't short circuit,
        // if no result was ready.
        if self.as_ref().get_ref().next_height_to_fetch().is_none() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.in_progress.len(), None)
    }
}

#[instrument(
    skip_all,
    fields(%height),
)]
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

#[cfg(test)]
mod tests {
    use futures_bounded::FuturesMap;
    use sequencer_client::tendermint::block::Height;

    use super::BlocksFromHeightStream;

    async fn make_stream() -> BlocksFromHeightStream {
        let pool = crate::client_provider::mock::TestPool::setup().await;
        BlocksFromHeightStream {
            next_expected_height: Height::from(1u32),
            greatest_requested_height: None,
            latest_sequencer_height: Height::from(2u32),
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), 10),
            pool: pool.pool.clone(),
            max_ahead: 3,
        }
    }

    #[tokio::test]
    async fn stream_next_blocks() {
        let mut stream = make_stream().await;
        assert_eq!(
            Some(stream.next_expected_height),
            stream.next_height_to_fetch(),
            "an unset greatest requested height should lead to the next expected height",
        );

        stream.greatest_requested_height = Some(Height::from(1u32));
        assert_eq!(
            Some(stream.latest_sequencer_height),
            stream.next_height_to_fetch(),
            "the greated requested height is right before the latest observed height, which \
             should give the observed height",
        );
        stream.greatest_requested_height = Some(Height::from(2u32));
        assert!(
            stream.next_height_to_fetch().is_none(),
            "the greatest requested height being the latest observed height should give nothing",
        );
        stream.greatest_requested_height = Some(Height::from(4u32));
        stream.latest_sequencer_height = Height::from(5u32);
        assert!(
            stream.next_height_to_fetch().is_none(),
            "a greatest height before the latest observed height but too far ahead of the next \
             expected height should give nothing",
        );
    }
}
