//! A task writing sequencer blocks to Celestia.
//!
//! [`BlobSubmitter`] receives [`SequencerBlock`]s over a channel,
//! converts them to Celestia [`Blob`]s, and writes them to Celestia
//! using the `blob.Submit` API.
//!
//! [`BlobSubmitter`] submits converted blobs strictly in the order it
//! receives blocks and imposes no extra ordering. This means that if
//! another task sends sequencer blocks ordered by their heights, then
//! they will be written in that order.
use std::{
    future::Future,
    mem,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_types::Blob;
use futures::{
    future::{
        BoxFuture,
        Fuse,
        FusedFuture as _,
    },
    stream::FuturesOrdered,
    FutureExt as _,
};
use pin_project_lite::pin_project;
use sequencer_client::{
    tendermint::block::Height as SequencerHeight,
    SequencerBlock,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            error::{
                SendError,
                TrySendError,
            },
        },
        watch,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
    Instrument,
    Span,
};

use super::{
    celestia_client::CelestiaClient,
    BuilderError,
    CelestiaClientBuilder,
    SubmissionState,
    TrySubmitError,
};
use crate::IncludeRollup;

mod conversion;

use conversion::{
    convert,
    ConversionInfo,
    Converted,
};

struct QueuedConvertedBlocks {
    // The maximum number of blobs permitted to sit in the blob queue.
    max_blobs: usize,
    blobs: Vec<Blob>,
    infos: Vec<ConversionInfo>,
    greatest_sequencer_height: Option<SequencerHeight>,
}

impl QueuedConvertedBlocks {
    fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    fn num_blobs(&self) -> usize {
        self.blobs.len()
    }

    fn num_converted(&self) -> usize {
        self.infos.len()
    }

    fn with_max_blobs(max_blobs: usize) -> Self {
        Self {
            max_blobs,
            blobs: Vec::new(),
            infos: Vec::new(),
            greatest_sequencer_height: None,
        }
    }

    fn has_capacity(&self) -> bool {
        self.blobs.len() < self.max_blobs
    }

    fn push(&mut self, mut converted: Converted) {
        self.blobs.append(&mut converted.blobs);
        let info = converted.info;
        let greatest_height = self
            .greatest_sequencer_height
            .get_or_insert(info.sequencer_height);
        *greatest_height = std::cmp::max(*greatest_height, info.sequencer_height);
        self.infos.push(info);
    }

    /// Lazily move the currently queued blobs out of the queue.
    ///
    /// The main reason for this method to exist is to work around async-cancellation.
    /// Only when the returned [`TakeQueued`] future is polled are the blobs moved
    /// out of the queue.
    fn take(&mut self) -> TakeQueued<'_> {
        TakeQueued {
            inner: Some(self),
        }
    }
}

pin_project! {
    struct TakeQueued<'a> {
        inner: Option<&'a mut QueuedConvertedBlocks>,
    }
}

impl<'a> Future for TakeQueued<'a> {
    type Output = Option<QueuedConvertedBlocks>;

    fn poll(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let queued = self
            .project()
            .inner
            .take()
            .expect("this future must not be polled twice");
        let empty = QueuedConvertedBlocks::with_max_blobs(queued.max_blobs);
        let queued = mem::replace(queued, empty);
        if queued.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some(queued))
        }
    }
}

#[derive(Clone)]
pub(super) struct BlobSubmitterHandle {
    tx: mpsc::Sender<SequencerBlock>,
}

impl BlobSubmitterHandle {
    /// Send a block to the blob submitter immediately.
    ///
    /// This is a thin wrapper around [`mpsc::Sender::try_send`].
    // allow: just forwarding the error type
    #[allow(clippy::result_large_err)]
    pub(super) fn try_send(
        &self,
        block: SequencerBlock,
    ) -> Result<(), TrySendError<SequencerBlock>> {
        self.tx.try_send(block)
    }

    /// Sends a block to the blob submitter.
    ///
    /// This is a thin wrapper around [`mpsc::Sender::send`].
    // allow: just forwarding the error type
    #[allow(clippy::result_large_err)]
    pub(super) async fn send(
        &self,
        block: SequencerBlock,
    ) -> Result<(), SendError<SequencerBlock>> {
        self.tx.send(block).await
    }
}

pub(super) struct BlobSubmitter {
    /// The builder for a client to submit blobs to Celestia.
    client_builder: CelestiaClientBuilder,

    /// The rollups whose data should be included in submissions.
    rollup_filter: IncludeRollup,

    /// The channel over which sequencer blocks are received.
    blocks: mpsc::Receiver<SequencerBlock>,

    /// The collection of tasks converting from sequencer blocks to celestia blobs,
    /// with the sequencer blocks' heights used as keys.
    conversions: Conversions,

    /// Celestia blobs waiting to be submitted after conversion from sequencer blocks.
    blobs: QueuedConvertedBlocks,

    /// The state of the relayer.
    state: Arc<super::State>,

    /// Tracks the submission state and writes it to disk before and after each Celestia
    /// submission.
    submission_state: SubmissionState,

    /// The shutdown token to signal that blob submitter should finish its current submission and
    /// exit.
    shutdown_token: CancellationToken,
}

impl BlobSubmitter {
    pub(super) fn new(
        client_builder: CelestiaClientBuilder,
        rollup_filter: IncludeRollup,
        state: Arc<super::State>,
        submission_state: SubmissionState,
        shutdown_token: CancellationToken,
    ) -> (Self, BlobSubmitterHandle) {
        // XXX: The channel size here is just a number. It should probably be based on some
        // heuristic about the number of expected blobs in a block.
        let (tx, rx) = mpsc::channel(128);
        let submitter = Self {
            client_builder,
            rollup_filter,
            blocks: rx,
            conversions: Conversions::new(8),
            blobs: QueuedConvertedBlocks::with_max_blobs(128),
            state,
            submission_state,
            shutdown_token,
        };
        let handle = BlobSubmitterHandle {
            tx,
        };
        (submitter, handle)
    }

    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let init_result = select!(
            () = self.shutdown_token.cancelled() => return Ok(()),
            init_result = init_with_retry(self.client_builder.clone()) => init_result,
        );
        let client = init_result.map_err(|error| {
            let message = "failed to initialize celestia client";
            error!(%error, message);
            error.wrap_err(message)
        })?;

        let mut submission = Fuse::terminated();

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("shutdown signal received");
                    break Ok("received shutdown signal");
                }

                // handle result of submitting blocks to Celestia, if in flight
                submission_result = &mut submission, if !submission.is_terminated() => {
                    // XXX: Breaks the select-loop and returns. With the current retry-logic in
                    // `submit_blobs` this happens after u32::MAX retries which is effectively never.
                    // self.submission_state = match submission_result.wrap_err("failed submitting blocks to Celestia")
                    self.submission_state = match submission_result {
                        Ok(state) => state,
                        Err(err) => {
                            // Use `wrap_err` on the return break value. Using it on the match-value causes
                            // type inference to fail.
                            break Err(err).wrap_err("failed submitting blocks to Celestia");
                        }
                    };
                }

                // submit blocks to Celestia, if no submission in flight
                Some(blobs) = self.blobs.take(), if submission.is_terminated() => {
                    submission = submit_blobs(
                        client.clone(),
                        blobs,
                        self.state.clone(),
                        self.submission_state.clone(),
                    ).boxed().fuse();
                }

                // handle result of converting blocks to blobs
                Some((sequencer_height, conversion_result)) = self.conversions.next() => {
                     match conversion_result {
                        // XXX: Emitting at ERROR level because failing to convert constitutes
                        // a fundamental problem for the relayer, even though it can happily
                        // continue chugging along.
                        // XXX: Should there instead be a mechanism to bubble up the error and
                        // have sequencer-relayer return with an error code (so that k8s can halt
                        // the chain)? This should probably be part of the protocol/sequencer
                        // proper.
                        Err(error) => error!(
                            %sequencer_height,
                            %error,
                            "failed converting sequencer blocks to celestia blobs",
                        ),
                        Ok(converted) => self.blobs.push(converted),
                    };
                }

                // enqueue new blocks for conversion to blobs if there is capacity
                Some(block) = self.blocks.recv(), if self.has_capacity() => {
                    debug!(
                        height = %block.height(),
                        "received sequencer block for submission",
                    );
                    self.conversions.push(block, self.rollup_filter.clone());
                }

            );
        };

        match &reason {
            Ok(reason) => info!(reason, "starting shutdown"),
            Err(reason) => error!(%reason, "starting shutdown"),
        }

        if submission.is_terminated() {
            info!("no submissions to Celestia were in flight, exiting now");
        } else {
            info!("a submission to Celestia is in flight; waiting for it to finish");
            if let Err(error) = submission.await {
                error!(%error, "last submission to Celestia failed before exiting");
            }
        }
        reason.map(|_| ())
    }

    /// Returns if the submitter has capacity for more blocks.
    fn has_capacity(&self) -> bool {
        self.conversions.has_capacity() && self.blobs.has_capacity()
    }
}

/// Submits new blobs Celestia.
///
/// # Panics
/// Panics if `blocks` is empty. This function should only be called if there is something to
/// submit.
#[instrument(skip_all)]
async fn submit_blobs(
    client: CelestiaClient,
    blocks: QueuedConvertedBlocks,
    state: Arc<super::State>,
    submission_state: SubmissionState,
) -> eyre::Result<SubmissionState> {
    info!(
        blocks = %telemetry::display::json(&blocks.infos),
        "initiated submission of sequencer blocks converted to Celestia blobs",
    );

    let start = std::time::Instant::now();

    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_COUNT).increment(1);
    // XXX: The number of sequencer blocks per celestia tx is equal to the number of heights passed
    // into this function. This comes from the way that `QueuedBlocks::take` is implemented.
    //
    // allow: the number of blocks should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blocks_per_celestia_tx = blocks.num_converted() as f64;
    metrics::gauge!(crate::metrics_init::BLOCKS_PER_CELESTIA_TX).set(blocks_per_celestia_tx);

    // allow: the number of blobs should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blobs_per_celestia_tx = blocks.num_blobs() as f64;
    metrics::gauge!(crate::metrics_init::BLOBS_PER_CELESTIA_TX).set(blobs_per_celestia_tx);

    let largest_height = blocks.greatest_sequencer_height.expect(
        "there should always be blobs and accompanying sequencer heights when this function is \
         called",
    );

    let submission_started = match crate::utils::flatten(
        tokio::task::spawn_blocking(move || submission_state.initialize(largest_height))
            .in_current_span()
            .await,
    ) {
        Err(error) => {
            error!(%error, "failed to initialize submission; abandoning");
            return Err(error);
        }
        Ok(state) => state,
    };

    let celestia_height = match submit_with_retry(client, blocks.blobs, state.clone()).await {
        Err(error) => {
            let message = "failed submitting blobs to Celestia";
            error!(%error, message);
            return Err(error.wrap_err(message));
        }
        Ok(height) => height,
    };
    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_HEIGHT).absolute(celestia_height);
    metrics::histogram!(crate::metrics_init::CELESTIA_SUBMISSION_LATENCY).record(start.elapsed());

    info!(%celestia_height, "successfully submitted blobs to Celestia");

    state.set_celestia_connected(true);
    state.set_latest_confirmed_celestia_height(celestia_height);

    let final_state = match crate::utils::flatten(
        tokio::task::spawn_blocking(move || submission_started.finalize(celestia_height))
            .in_current_span()
            .await,
    ) {
        Err(error) => {
            error!(%error, "failed to finalize submission; abandoning");
            return Err(error);
        }
        Ok(state) => state,
    };
    Ok(final_state)
}

#[instrument(skip_all)]
async fn init_with_retry(client_builder: CelestiaClientBuilder) -> eyre::Result<CelestiaClient> {
    let span = Span::current();

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_secs(1))
        .max_delay(Duration::from_secs(30))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &BuilderError| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = %eyre::Report::new(error.clone()),
                    "failed to initialize celestia client; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let celestia_client = tryhard::retry_fn(move || client_builder.clone().try_build())
        .with_config(retry_config)
        .in_current_span()
        .await
        .wrap_err("retry attempts exhausted; bailing")?;
    info!("initialized celestia client");
    Ok(celestia_client)
}

async fn submit_with_retry(
    client: CelestiaClient,
    blobs: Vec<Blob>,
    state: Arc<super::State>,
) -> eyre::Result<u64> {
    // Moving the span into `on_retry`, because tryhard spawns these in a tokio
    // task, losing the span.
    let span = Span::current();

    // Create a watch channel to allow the `on_retry` function to provide the received
    // `TrySubmitError` to the next attempt of the `retry_fn`.
    let (last_error_sender, last_error_receiver) = watch::channel(None);

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        // 12 seconds is the Celestia block time
        .max_delay(Duration::from_secs(12))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &TrySubmitError| {
                metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_FAILURE_COUNT)
                    .increment(1);

                let state = Arc::clone(&state);
                state.set_celestia_connected(false);
                let _ = last_error_sender.send(Some(error.clone()));

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);

                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = %eyre::Report::new(error.clone()),
                    "failed submitting blobs to Celestia; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let blobs = Arc::new(blobs);

    let height = tryhard::retry_fn(move || {
        client
            .clone()
            .try_submit(blobs.clone(), last_error_receiver.clone())
    })
    .with_config(retry_config)
    .in_current_span()
    .await
    .wrap_err("retry attempts exhausted; bailing")?;
    Ok(height)
}

/// Currently running conversions of Sequencer blocks to Celestia blobs.
///
/// The conversion result will be returned in the order they are pushed
/// into this queue.
///
/// Note on the implementation: the conversions are done in a blocking tokio
/// task so that conversions are started immediately without needing extra
/// polling. This means that the only contribution that `FuturesOrdered`
/// provides is ordering the conversion result by the order the blocks are
/// received. This however is desirable because we want to submit sequencer
/// blocks in the order of their heights to Celestia.
struct Conversions {
    // The currently active conversions.
    active: FuturesOrdered<BoxFuture<'static, (SequencerHeight, eyre::Result<Converted>)>>,

    // The maximum number of conversions that can be active at the same time.
    max_conversions: usize,
}

impl Conversions {
    fn new(max_conversions: usize) -> Self {
        Self {
            active: FuturesOrdered::new(),
            max_conversions,
        }
    }

    fn has_capacity(&self) -> bool {
        self.active.len() < self.max_conversions
    }

    fn push(&mut self, block: SequencerBlock, rollup_filter: IncludeRollup) {
        let height = block.height();
        let conversion = tokio::task::spawn_blocking(move || convert(block, rollup_filter));
        let fut = async move {
            let res = crate::utils::flatten(conversion.await);
            (height, res)
        }
        .boxed();
        self.active.push_back(fut);
    }

    async fn next(&mut self) -> Option<(SequencerHeight, eyre::Result<Converted>)> {
        use tokio_stream::StreamExt as _;
        self.active.next().await
    }
}
