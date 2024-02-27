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
    debug,
    instrument,
    warn,
    Span,
};

#[derive(Default, Debug)]
struct Heights {
    last_observed: Option<Height>,
    last_requested: Option<Height>,
}

impl Heights {
    /// Returns the next height to be fetched.
    ///
    /// If `last_observed` is unset, returns `None`.
    /// If `last_requested` is unset, returns `last_observed`.
    /// If both are set, returns `last_requested + 1` if less than
    /// `last_observed`, `None` otherwise.
    fn next_height_to_fetch(&self) -> Option<Height> {
        let last_observed = self.last_observed?;
        match self.last_requested {
            None => Some(last_observed),
            Some(last_requested) if last_requested < last_observed => {
                Some(last_requested.increment())
            }
            Some(..) => None,
        }
    }
}

pin_project! {
    pub(super) struct BlockStream {
        client: HttpClient,
        heights: Heights,
        #[pin]
        future: Option<BoxFuture<'static, eyre::Result<SequencerBlock>>>,
        paused: bool,
        block_time: Duration,
        state: Arc<super::State>,
    }
}

impl BlockStream {
    pub(super) fn new(client: HttpClient, state: Arc<super::State>) -> Self {
        Self {
            client,
            heights: Heights::default(),
            future: None,
            block_time: Duration::from_millis(1_000),
            paused: false,
            state,
        }
    }

    pub(super) fn set_block_time(&mut self, block_time: Duration) {
        self.block_time = block_time;
    }

    pub(super) fn set_latest_sequencer_height(&mut self, height: Height) {
        self.heights.last_observed.replace(height);
    }

    pub(super) fn pause(&mut self) {
        debug!("stream paused");
        self.paused = true;
    }

    pub(super) fn resume(&mut self) {
        debug!("stream resumed");
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
                    .heights
                    .last_requested
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
                this.heights.last_requested.replace(height);
            } else {
                break None;
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let latest_height = self.heights.last_observed.map_or(0, |h| h.value());
        let requested_height = self.heights.last_requested.map_or(0, |h| h.value());
        let mut lower_limit = latest_height
            .saturating_sub(requested_height)
            .try_into()
            .expect(
                "height differences should convert to usize on all reasonable architectures; 64 \
                 bit: heights are represented as u64; 32 bit: cometbft heights are non-negative \
                 i64",
            );
        // Add 1 if a fetch is in-flight. Example: if requested and latest heights are
        // both 10, then (latest - requested) = 0. But that is only true if the future already
        // resolved and its result was returned. If requested height is 9 and latest is 10,
        // then `(latest - requested) = 1`, meaning there is one more height to be fetched plus
        // the one that's currently in-flight.
        lower_limit += Into::<usize>::into(self.future.is_some());
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
    .await
    .wrap_err("retry attempts exhausted; bailing")?;

    state.set_sequencer_connected(true);

    Ok(block)
}
