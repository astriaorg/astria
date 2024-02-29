use std::{
    future::Future,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::{
    celestia_types::Blob,
    jsonrpsee::http_client::HttpClient,
    submission::ToBlobs as _,
};
use futures::{
    future::{
        Fuse,
        FusedFuture as _,
    },
    FutureExt as _,
};
use pin_project_lite::pin_project;
use sequencer_client::{
    tendermint::block::Height as SequencerHeight,
    SequencerBlock,
};
use tokio::{
    select,
    sync::mpsc::{
        self,
        error::{
            SendError,
            TrySendError,
        },
        Receiver,
        Sender,
    },
};
use tokio_util::task::JoinMap;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
    Instrument as _,
    Span,
};

struct QueuedBlobs {
    blobs: Vec<Blob>,
    heights: Vec<SequencerHeight>,
}

impl QueuedBlobs {
    fn num_blobs(&self) -> usize {
        self.blobs.len()
    }

    fn new() -> Self {
        Self {
            heights: Vec::new(),
            blobs: Vec::new(),
        }
    }

    fn push(&mut self, mut blobs: Vec<Blob>, height: SequencerHeight) {
        self.blobs.append(&mut blobs);
        self.heights.push(height);
    }

    /// Lazily move the currently queued blobs out of the queue.
    ///
    /// The main reason for this method to exist is to work around async-cancellation.
    /// Only when the returned [`TakeBlobs`] future is polled are the blobs moved
    /// out of the queue.
    fn take(&mut self) -> TakeBlobs<'_> {
        TakeBlobs {
            queue: Some(self),
        }
    }
}

pin_project! {
    struct TakeBlobs<'a> {
        queue: Option<&'a mut QueuedBlobs>,
    }
}

impl<'a> Future for TakeBlobs<'a> {
    type Output = Option<(Vec<Blob>, Vec<SequencerHeight>)>;

    fn poll(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let queue = self
            .project()
            .queue
            .take()
            .expect("this future must not be polled twice");
        let blobs = std::mem::take(&mut queue.blobs);
        let heights = std::mem::take(&mut queue.heights);
        if blobs.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some((blobs, heights)))
        }
    }
}

#[derive(Clone)]
pub(super) struct BlobSubmitterHandle {
    tx: Sender<SequencerBlock>,
}

impl BlobSubmitterHandle {
    /// Send a block to the blob submitter immediately.
    ///
    /// This is a thin wrapper around [`Sender::try_send`].
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
    /// This is a thin wrapper around [`Sender::send`].
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
    // The client to submit blobs to Celestia.
    client: HttpClient,

    // The channel over which sequencer blocks are received.
    blocks: Receiver<SequencerBlock>,

    // The maximum number of conversions that can be active at the same time.
    // Submitter will refuse to accept more blocks until there are fewer active
    // conversions.
    max_conversions: usize,

    // The collection of tasks converting from sequencer blocks to celestia blobs,
    // with the sequencer blocks' heights used as keys.
    conversions: JoinMap<SequencerHeight, eyre::Result<Vec<Blob>>>,

    // The maximum number of blobs permitted to sit in the blob queue.
    // Submitter will refuse to accept more blocks until the queue is freed up again.
    max_blobs: usize,

    // Celestia blobs waiting to be submitted after conversion from sequencer blocks.
    blobs: QueuedBlobs,

    // The state of the relayer.
    state: Arc<super::State>,
}

impl BlobSubmitter {
    pub(super) fn new(client: HttpClient, state: Arc<super::State>) -> (Self, BlobSubmitterHandle) {
        // XXX: The channel size here is just a number. It should probably be based on some
        // heuristic about the number of expected blobs in a block.
        let (tx, rx) = mpsc::channel(128);
        let submitter = Self {
            client,
            blocks: rx,
            max_conversions: 8,
            conversions: JoinMap::new(),
            max_blobs: 128,
            blobs: QueuedBlobs::new(),
            state,
        };
        let handle = BlobSubmitterHandle {
            tx,
        };
        (submitter, handle)
    }

    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let mut submission = Fuse::terminated();

        loop {
            select!(
                Some(block) = self.blocks.recv(), if self.has_capacity() => {
                    debug!(
                        height = %block.height(),
                        "received sequencer block for submission",
                    );
                    self.conversions.spawn_blocking(block.height(), move || convert(block));
                }

                Some((sequencer_height, conversion_result)) = self.conversions.join_next() => {
                     match crate::utils::flatten(conversion_result) {
                        // XXX: Emitting at ERROR level because failing to convert contitutes
                        // a fundamental problem for the relayer, even though it can happily
                        // continue chugging along.
                        // XXX: Should there instead be a mechanism to bubble up the error and
                        // have sequencer-relayer return with an error code (so that k8s can halt
                        // the chain)? This should probably be part of the protocal/sequencer proper.
                        Err(error) => error!(
                            %sequencer_height,
                            %error,
                            "failed converting sequencer blocks to celestia Blobs",
                        ),
                        Ok(blobs) => self.blobs.push(blobs, sequencer_height),
                    };
                }

                Some((blobs, heights)) = self.blobs.take(), if submission.is_terminated() => {
                    submission = submit_blobs(self.client.clone(), blobs, heights, self.state.clone()).boxed().fuse();
                }

                submission_result = &mut submission, if !submission.is_terminated() => {
                    // XXX: Breaks the select-loop and returns. With the current retry-logic in
                    // `submit_blobs` this happens after u32::MAX retries which is effectively never.
                    let height = submission_result.wrap_err("failed submitting blobs to Celestia")?;
                    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_HEIGHT).absolute(height);
                }
            );
        }
    }

    /// Returns if the submitter has capacity for more blocks.
    fn has_capacity(&self) -> bool {
        (self.conversions.len() < self.max_conversions) && self.blobs.num_blobs() < self.max_blobs
    }
}

fn convert(block: SequencerBlock) -> eyre::Result<Vec<Blob>> {
    let mut blobs = Vec::new();
    block
        .try_to_blobs(&mut blobs)
        .wrap_err("failed converting sequencer block to celestia blobs")?;
    Ok(blobs)
}

#[instrument(
    skip_all,
    fields(
        num_blobs = blobs.len(),
        sequencer_heights = %ReportSequencerHeights(&sequencer_heights),
))]
async fn submit_blobs(
    client: HttpClient,
    blobs: Vec<Blob>,
    sequencer_heights: Vec<SequencerHeight>,
    state: Arc<super::State>,
) -> eyre::Result<u64> {
    let height = match submit_with_retry(client, blobs, state.clone()).await {
        Err(error) => {
            let message = "failed submitting blobs to Celestia";
            error!(%error, message);
            return Err(error.wrap_err(message));
        }
        Ok(height) => height,
    };
    state.set_celestia_connected(true);
    state.set_latest_confirmed_celestia_height(height);
    info!(celestia_height = %height, "successfully submitted blocks to Celestia");
    Ok(height)
}

async fn submit_with_retry(
    client: HttpClient,
    blobs: Vec<Blob>,
    state: Arc<super::State>,
) -> eyre::Result<u64> {
    use celestia_client::{
        celestia_rpc::BlobClient as _,
        celestia_types::blob::SubmitOptions,
    };
    // Moving the span into `on_retry`, because tryhard spawns these in a tokio
    // task, losing the span.
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        // 12 seconds is the Celestia block time
        .max_delay(Duration::from_secs(12))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
                let state = Arc::clone(&state);
                state.set_celestia_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);

                metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_FAILURE_COUNT)
                    .increment(1);

                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    %error,
                    "failed submitting blobs to Celestia; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let blobs = Arc::new(blobs);
    let height = tryhard::retry_fn(move || {
        let client = client.clone();
        let blobs = blobs.clone();
        async move {
            client
                .blob_submit(
                    &blobs,
                    SubmitOptions {
                        fee: None,
                        gas_limit: None,
                    },
                )
                .await
                .wrap_err("failed submitting sequencer blocks to celestia")
        }
    })
    .with_config(retry_config)
    .in_current_span()
    .await
    .wrap_err("retry attempts exhausted; bailing")?;
    Ok(height)
}

struct ReportSequencerHeights<'a>(&'a [SequencerHeight]);

impl<'a> std::fmt::Display for ReportSequencerHeights<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write as _;
        f.write_char('[')?;
        let mut heights = self.0.iter();
        if let Some(height) = heights.next() {
            let mut buf = itoa::Buffer::new();
            f.write_str(buf.format(height.value()))?;
        }
        for height in heights {
            f.write_str(", ")?;
            let mut buf = itoa::Buffer::new();
            f.write_str(buf.format(height.value()))?;
        }
        f.write_char(']')?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ReportSequencerHeights,
        SequencerHeight,
    };

    #[track_caller]
    fn assert_block_height_formatting(heights: &[u32], expected: &str) {
        let blocks: Vec<_> = heights.iter().copied().map(SequencerHeight::from).collect();
        let actual = ReportSequencerHeights(&blocks).to_string();
        assert_eq!(&actual, expected);
    }

    #[test]
    fn reported_block_heights_formatting() {
        assert_block_height_formatting(&[], "[]");
        assert_block_height_formatting(&[1], "[1]");
        assert_block_height_formatting(&[4, 2, 1], "[4, 2, 1]");
    }
}
