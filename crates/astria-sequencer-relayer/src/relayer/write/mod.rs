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
    sync::Arc,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_types::Blob;
use futures::{
    future::{
        Fuse,
        FusedFuture as _,
    },
    FutureExt as _,
};
use sequencer_client::SequencerBlock;
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
use crate::{
    metrics_init,
    IncludeRollup,
};

mod conversion;
use conversion::NextSubmission;

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

    /// The channel over which sequencer blocks are received.
    blocks: mpsc::Receiver<SequencerBlock>,

    /// The accumulator of all data that will be submitted to Celestia on the next submission.
    next_submission: NextSubmission,

    /// The state of the relayer.
    state: Arc<super::State>,

    /// Tracks the submission state and writes it to disk before and after each Celestia
    /// submission.
    submission_state: SubmissionState,

    /// The shutdown token to signal that blob submitter should finish its current submission and
    /// exit.
    shutdown_token: CancellationToken,

    /// A block that could not be added to `next_submission` because it would overflow its
    /// hardcoded limit.
    pending_block: Option<SequencerBlock>,
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
            blocks: rx,
            next_submission: NextSubmission::new(rollup_filter),
            state,
            submission_state,
            shutdown_token,
            pending_block: None,
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

        // A submission to Celestia that is currently in-flight.
        let mut ongoing_submission = Fuse::terminated();

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("shutdown signal received");
                    break Ok("received shutdown signal");
                }

                // handle result of submitting blocks to Celestia, if in flight
                submission_result = &mut ongoing_submission,
                                    if !ongoing_submission.is_terminated()
                                    =>
                {
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
                Some(submission) = self.next_submission.take(),
                                    if ongoing_submission.is_terminated()
                                    => {
                    ongoing_submission = submit_blobs(
                        client.clone(),
                        submission,
                        self.state.clone(),
                        self.submission_state.clone(),
                    ).boxed().fuse();
                    if let Some(block) = self.pending_block.take() {
                        if let Err(error) = self.add_sequencer_block_to_next_submission(block) {
                            break Err(error).wrap_err(
                                "critically failed adding Sequencer block to next submission"
                            );
                        }
                    }
                }

                // add new blocks to the next submission if there is space.
                Some(block) = self.blocks.recv(), if self.has_capacity() => {
                    if let Err(error) = self.add_sequencer_block_to_next_submission(block) {
                        break Err(error).wrap_err(
                            "critically failed adding Sequencer block to next submission"
                        );
                    }
                }

            );
        };

        match &reason {
            Ok(reason) => info!(reason, "starting shutdown"),
            Err(reason) => error!(%reason, "starting shutdown"),
        }

        if ongoing_submission.is_terminated() {
            info!("no submissions to Celestia were in flight, exiting now");
        } else {
            info!("a submission to Celestia is in flight; waiting for it to finish");
            if let Err(error) = ongoing_submission.await {
                error!(%error, "last submission to Celestia failed before exiting");
            }
        }
        reason.map(|_| ())
    }

    #[instrument(skip_all, fields(sequencer_height = block.height().value()), err)]
    fn add_sequencer_block_to_next_submission(
        &mut self,
        block: SequencerBlock,
    ) -> eyre::Result<()> {
        match self.next_submission.try_add(block) {
            Ok(()) => debug!("block was scheduled for next submission"),
            Err(conversion::TryAddError::Full(block)) => {
                debug!(
                    "block was rejected from next submission because it would overflow the \
                     maximum payload size; pushing back until the next submission is done"
                );
                self.pending_block = Some(*block);
            }
            Err(err) => {
                return Err(err).wrap_err("failed adding sequencer block to next submission");
            }
        }
        Ok(())
    }

    /// Returns if the next submission still has capacity.
    fn has_capacity(&self) -> bool {
        // The next submission has capacity if no block was rejected.
        self.pending_block.is_none()
    }
}

/// Submits new blobs Celestia.
#[instrument(skip_all)]
async fn submit_blobs(
    client: CelestiaClient,
    data: conversion::Submission,
    state: Arc<super::State>,
    submission_state: SubmissionState,
) -> eyre::Result<SubmissionState> {
    info!(
        blocks = %telemetry::display::json(&data.input_metadata()),
        total_data_uncompressed_size = data.uncompressed_size(),
        total_data_compressed_size = data.compressed_size(),
        compression_ratio = data.compression_ratio(),
        "initiated submission of sequencer blocks converted to Celestia blobs",
    );

    let start = std::time::Instant::now();

    // allow: gauges require f64, it's okay if the metrics get messed up by overflow or precision
    // loss
    #[allow(clippy::cast_precision_loss)]
    let compressed_size = data.compressed_size() as f64;
    metrics::gauge!(metrics_init::TOTAL_BLOB_DATA_SIZE_FOR_ASTRIA_BLOCK).set(compressed_size);

    metrics::gauge!(metrics_init::COMPRESSION_RATIO_FOR_ASTRIA_BLOCK).set(data.compression_ratio());

    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_COUNT).increment(1);
    // XXX: The number of sequencer blocks per celestia tx is equal to the number of heights passed
    // into this function. This comes from the way that `QueuedBlocks::take` is implemented.
    //
    // allow: the number of blocks should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blocks_per_celestia_tx = data.num_blocks() as f64;
    metrics::gauge!(crate::metrics_init::BLOCKS_PER_CELESTIA_TX).set(blocks_per_celestia_tx);

    // allow: the number of blobs should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blobs_per_celestia_tx = data.num_blobs() as f64;
    metrics::gauge!(crate::metrics_init::BLOBS_PER_CELESTIA_TX).set(blobs_per_celestia_tx);

    let largest_sequencer_height = data.greatest_sequencer_height();
    let blobs = data.into_blobs();

    let submission_started = match crate::utils::flatten(
        tokio::task::spawn_blocking(move || submission_state.initialize(largest_sequencer_height))
            .in_current_span()
            .await,
    ) {
        Err(error) => {
            error!(%error, "failed to initialize submission; abandoning");
            return Err(error);
        }
        Ok(state) => state,
    };

    let celestia_height = match submit_with_retry(client, blobs, state.clone()).await {
        Err(error) => {
            let message = "failed submitting blobs to Celestia";
            error!(%error, message);
            return Err(error.wrap_err(message));
        }
        Ok(height) => height,
    };
    metrics::counter!(crate::metrics_init::SEQUENCER_SUBMISSION_HEIGHT)
        .absolute(largest_sequencer_height.value());
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
