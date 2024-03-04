//! A stream of sequencer blocks.
use std::{
    future::Future as _,
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    Report,
    WrapErr as _,
};
use futures::{
    future::BoxFuture,
    ready,
    FutureExt as _,
};
use pin_project_lite::pin_project;
use sequencer_client::{
    tendermint::block::Height,
    HttpClient,
    SequencerBlock,
};
use tokio_stream::Stream;
use tracing::{
    info,
    instrument,
    warn,
    Instrument as _,
    Span,
};

#[derive(Debug)]
struct Heights {
    last_observed: Option<Height>,
    next: Height,
}

impl Heights {
    /// Returns the next height to be fetched.
    ///
    /// Returns `None` if `last_observed` is unset or if `next` is greater than or equal to
    /// `last_observed`.
    /// Returns `next` otherwise.
    fn next_height_to_fetch(&self) -> Option<Height> {
        let last_observed = self.last_observed?;
        if self.next <= last_observed {
            Some(self.next)
        } else {
            None
        }
    }

    fn increment_next(&mut self) {
        self.next = self.next.increment();
    }
}

pin_project! {
    pub(super) struct BlockStream {
        client: HttpClient,
        heights: Heights,
        #[pin]
        future: Option<BoxFuture<'static, eyre::Result<SequencerBlock>>>,
        height_in_flight: Option<Height>,
        paused: bool,
        block_time: Duration,
        state: Arc<super::State>,
    }
}

impl BlockStream {
    pub(super) fn builder() -> BlockStreamBuilder {
        BlockStreamBuilder::new()
    }

    pub(super) fn set_latest_sequencer_height(&mut self, height: Height) {
        self.heights.last_observed.replace(height);
    }

    pub(super) fn pause(&mut self) {
        self.paused = true;
    }

    pub(super) fn resume(&mut self) {
        self.paused = false;
    }
}

impl Stream for BlockStream {
    type Item = (Height, eyre::Result<SequencerBlock>);

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.poll(cx));
                let height = this
                    .height_in_flight
                    .take()
                    .expect("must be set if a future was scheduled");
                this.future.set(None);
                break Some((height, item));
            } else if !*this.paused && this.heights.next_height_to_fetch().is_some() {
                // XXX: this can be expressed more concisely once if-let-chains are stabilized.
                // But `next_height_to_fetch` is cheap so it's fine doing it twice.
                let height = this
                    .heights
                    .next_height_to_fetch()
                    .expect("the if condition has assured that there is a height");
                this.future.set(Some(
                    fetch_block(
                        this.client.clone(),
                        height,
                        *this.block_time,
                        this.state.clone(),
                    )
                    .boxed(),
                ));
                this.state
                    .set_latest_requested_sequencer_height(height.value());
                this.height_in_flight.replace(height);
                this.heights.increment_next();
            } else {
                break None;
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let latest_height = self.heights.last_observed.map_or(0, |h| h.value());
        let next_height = self.heights.next.value();
        let mut lower_limit = latest_height.saturating_sub(next_height);

        // A new future will be spawned as long as next_heigth <= latest_height. So
        // 10 - 10 = 0, but there 1 future stil to be spawned at next height = 10.
        lower_limit += Into::<u64>::into(next_height <= latest_height);

        // Add 1 if a fetch is in-flight. Example: if requested and latest heights are
        // both 10, then (latest - requested) = 0. But that is only true if the future already
        // resolved and its result was returned. If requested height is 9 and latest is 10,
        // then `(latest - requested) = 1`, meaning there is one more height to be fetched plus
        // the one that's currently in-flight.
        lower_limit += Into::<u64>::into(self.future.is_some());

        let lower_limit = lower_limit.try_into().expect(
            "height differences should convert to usize on all reasonable architectures; 64 bit: \
             heights are represented as u64; 32 bit: cometbft heights are non-negative i64",
        );
        (lower_limit, None)
    }
}

/// Fetch the sequencer block at `height`.
///
/// If fetching the block fails, then a new fetch is scheduled with exponential backoff,
/// up to a maximum of `block_time` duration between subsequent requests.
#[instrument(skip_all, fields(%height))]
async fn fetch_block(
    client: HttpClient,
    height: Height,
    block_time: Duration,
    state: Arc<super::State>,
) -> eyre::Result<SequencerBlock> {
    use sequencer_client::SequencerClientExt as _;

    // Moving the span into `on_retry`, because tryhard spawns these in a tokio
    // task, losing the span.
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(block_time)
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
                metrics::counter!(crate::metrics_init::SEQUENCER_BLOCK_FETCH_FAILURE_COUNT)
                    .increment(1);

                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);

                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    %error,
                    "failed fetching block from sequencer; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let block = tryhard::retry_fn(move || {
        let client = client.clone();
        async move { client.sequencer_block(height).await.map_err(Report::new) }
    })
    .with_config(retry_config)
    .in_current_span()
    .await
    .wrap_err("retry attempts exhausted; bailing")?;

    state.set_sequencer_connected(true);

    Ok(block)
}

pub(super) struct NoBlockTime;
pub(super) struct WithBlockTime(Duration);
pub(super) struct NoClient;
pub(super) struct WithClient(HttpClient);
pub(super) struct NoState;
pub(super) struct WithState(Arc<super::State>);

pub(super) struct BlockStreamBuilder<TBlockTime = NoBlockTime, TClient = NoClient, TState = NoState>
{
    block_time: TBlockTime,
    client: TClient,
    last_fetched_height: Option<Height>,
    state: TState,
}

impl<TBlockTime, TClient, TState> BlockStreamBuilder<TBlockTime, TClient, TState> {
    pub(super) fn block_time(
        self,
        block_time: Duration,
    ) -> BlockStreamBuilder<WithBlockTime, TClient, TState> {
        let Self {
            client,
            last_fetched_height,
            state,
            ..
        } = self;
        BlockStreamBuilder {
            block_time: WithBlockTime(block_time),
            client,
            last_fetched_height,
            state,
        }
    }

    pub(super) fn client(
        self,
        client: HttpClient,
    ) -> BlockStreamBuilder<TBlockTime, WithClient, TState> {
        let Self {
            block_time,
            last_fetched_height,
            state,
            ..
        } = self;
        BlockStreamBuilder {
            block_time,
            client: WithClient(client),
            last_fetched_height,
            state,
        }
    }

    pub(super) fn set_last_fetched_height(
        self,
        last_fetched_height: Option<Height>,
    ) -> BlockStreamBuilder<TBlockTime, TClient, TState> {
        let Self {
            block_time,
            client,
            state,
            ..
        } = self;
        BlockStreamBuilder {
            block_time,
            client,
            last_fetched_height,
            state,
        }
    }

    pub(super) fn state(
        self,
        state: Arc<super::State>,
    ) -> BlockStreamBuilder<TBlockTime, TClient, WithState> {
        let Self {
            block_time,
            client,
            last_fetched_height,
            ..
        } = self;
        BlockStreamBuilder {
            block_time,
            client,
            last_fetched_height,
            state: WithState(state),
        }
    }
}

impl BlockStreamBuilder {
    fn new() -> Self {
        BlockStreamBuilder {
            block_time: NoBlockTime,
            client: NoClient,
            last_fetched_height: None,
            state: NoState,
        }
    }
}

impl BlockStreamBuilder<WithBlockTime, WithClient, WithState> {
    pub(super) fn build(self) -> BlockStream {
        let Self {
            block_time: WithBlockTime(block_time),
            client: WithClient(client),
            last_fetched_height,
            state: WithState(state),
        } = self;
        let next = match last_fetched_height {
            None => {
                let next = Height::from(1u32);
                info!(
                    "last fetched height was not set, so next height fetched from sequencer will \
                     be `{next}`"
                );
                next
            }
            Some(last_fetched) => {
                let next = last_fetched.increment();
                info!(
                    "last fetched height was set to `{last_fetched}`, so next height fetched from \
                     sequencer will be `{next}`"
                );
                next
            }
        };
        BlockStream {
            client,
            heights: Heights {
                last_observed: None,
                next,
            },
            future: None,
            height_in_flight: None,
            block_time,
            paused: false,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Height,
        Heights,
    };

    #[track_caller]
    fn assert_next_height_is_expected(
        last_observed: Option<u32>,
        next: u32,
        expected: Option<u32>,
    ) {
        let heights = Heights {
            last_observed: last_observed.map(Height::from),
            next: Height::from(next),
        };
        let expected = expected.map(Height::from);
        assert_eq!(expected, heights.next_height_to_fetch());
    }

    #[test]
    fn next_heights() {
        assert_next_height_is_expected(None, 1, None);
        assert_next_height_is_expected(Some(1), 1, Some(1));
        assert_next_height_is_expected(Some(2), 1, Some(1));
        assert_next_height_is_expected(Some(1), 2, None);
    }
}
