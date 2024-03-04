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
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
    task::Poll,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use celestia_client::{
    celestia_types::Blob,
    jsonrpsee::http_client::HttpClient,
    submission::ToBlobs as _,
};
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
use serde::{
    Deserialize,
    Serialize,
};
use tokio::{
    io::AsyncWriteExt,
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
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
    Instrument as _,
    Span,
};

pub(super) fn are_states_consistent(pre: PreSubmissionState, post: PostSubmissionState) -> bool {
    pre.sequencer_height == post.sequencer_height
        && pre.celestia_height_at_init < post.celestia_height_at_submit
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) struct PreSubmissionState {
    celestia_height_at_init: u64,
    sequencer_height: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) struct PostSubmissionState {
    celestia_height_at_submit: u64,
    sequencer_height: u64,
}

impl PostSubmissionState {
    pub(super) fn sequencer_height(&self) -> u64 {
        self.sequencer_height
    }
}

impl PreSubmissionState {
    fn into_submitted(self, celestia_height_at_submit: u64) -> PostSubmissionState {
        PostSubmissionState {
            celestia_height_at_submit,
            sequencer_height: self.sequencer_height,
        }
    }
}

struct QueuedBlobs {
    // The maximum number of blobs permitted to sit in the blob queue.
    max_blobs: usize,
    blobs: Vec<Blob>,
    heights: Vec<SequencerHeight>,
}

impl QueuedBlobs {
    fn new(max_blobs: usize) -> Self {
        Self {
            max_blobs,
            heights: Vec::new(),
            blobs: Vec::new(),
        }
    }

    fn has_capacity(&self) -> bool {
        self.blobs.len() < self.max_blobs
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

    // The collection of tasks converting from sequencer blocks to celestia blobs,
    // with the sequencer blocks' heights used as keys.
    conversions: Conversions,

    // Celestia blobs waiting to be submitted after conversion from sequencer blocks.
    blobs: QueuedBlobs,

    // The state of the relayer.
    state: Arc<super::State>,

    // The path to which blob submitter will write the submission state before attempting to submit
    // to Celestia
    presubmit_path: PathBuf,

    // The path to which blob submitter will write the submission state after successfully
    // submitting to Celestia
    postsubmit_path: PathBuf,
}

impl BlobSubmitter {
    pub(super) fn new(
        client: HttpClient,
        state: Arc<super::State>,
        presubmit_path: PathBuf,
        postsubmit_path: PathBuf,
    ) -> (Self, BlobSubmitterHandle) {
        // XXX: The channel size here is just a number. It should probably be based on some
        // heuristic about the number of expected blobs in a block.
        let (tx, rx) = mpsc::channel(128);
        let submitter = Self {
            client,
            blocks: rx,
            conversions: Conversions::new(8),
            blobs: QueuedBlobs::new(128),
            state,
            presubmit_path,
            postsubmit_path,
        };
        let handle = BlobSubmitterHandle {
            tx,
        };
        (submitter, handle)
    }

    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let mut submission = Fuse::terminated();

        // Fetch the latest Celestia height so there is a starting point when tracking
        // submissions.
        fetch_latest_celestia_height(self.client.clone(), self.state.clone())
            .await
            .wrap_err("failed fetching latest celestia height after many retries")?;

        let mut latest_submission = None;
        loop {
            select!(
                Some(block) = self.blocks.recv(), if self.has_capacity() => {
                    debug!(
                        height = %block.height(),
                        "received sequencer block for submission",
                    );
                    self.conversions.push(block);
                }

                Some((sequencer_height, conversion_result)) = self.conversions.next() => {
                     match conversion_result {
                        // XXX: Emitting at ERROR level because failing to convert contitutes
                        // a fundamental problem for the relayer, even though it can happily
                        // continue chugging along.
                        // XXX: Should there instead be a mechanism to bubble up the error and
                        // have sequencer-relayer return with an error code (so that k8s can halt
                        // the chain)? This should probably be part of the protocal/sequencer
                        // proper.
                        Err(error) => error!(
                            %sequencer_height,
                            %error,
                            "failed converting sequencer blocks to celestia blobs",
                        ),
                        Ok(blobs) => self.blobs.push(blobs, sequencer_height),
                    };
                }

                Some((blobs, heights)) = self.blobs.take(), if submission.is_terminated() => {
                    submission = submit_blobs(
                        self.client.clone(),
                        blobs,
                        heights,
                        self.state.clone(),
                        self.presubmit_path.clone(),
                        self.postsubmit_path.clone(),
                    ).boxed().fuse();
                }

                submission_result = &mut submission, if !submission.is_terminated() => {
                    // XXX: Breaks the select-loop and returns. With the current retry-logic in
                    // `submit_blobs` this happens after u32::MAX retries which is effectively never.
                    let new_submission = submission_result.wrap_err("failed submitting blobs to Celestia")?;
                    latest_submission.replace(new_submission);
                }
            );
        }
    }

    /// Returns if the submitter has capacity for more blocks.
    fn has_capacity(&self) -> bool {
        self.conversions.has_capacity() && self.blobs.has_capacity()
    }
}

/// Submits new blobs Celestia.
///
/// # Panics
/// Panics if `blobs` or `sequencer_heights` are empty. This function
/// should only be called if there is something to submit.
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
    presubmit_path: PathBuf,
    postsubmit_path: PathBuf,
) -> eyre::Result<PostSubmissionState> {
    let start = std::time::Instant::now();

    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_COUNT).increment(1);
    // XXX: The number of blocks per celestia tx is equal to the number of heights passed
    // into this function. This comes from the way that `QueuedBlocks::take` is implemented.
    //
    // allow: the number of blocks should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blocks_per_celestia_tx = sequencer_heights.len() as f64;
    metrics::gauge!(crate::metrics_init::BLOCKS_PER_CELESTIA_TX).set(blocks_per_celestia_tx);

    let latest_celestia_height = state
        .get_latest_confirmed_celestia_height()
        .ok_or_else(|| {
            eyre!(
                "internal state tracking latest celestia height was unexpectedtly unset, which \
                 should never happen"
            )
        })?;

    // allow: the number of blobs should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let blobs_per_celestia_tx = blobs.len() as f64;
    metrics::gauge!(crate::metrics_init::BLOBS_PER_CELESTIA_TX).set(blobs_per_celestia_tx);

    let presubmit_state =
        match initialize_submission(&sequencer_heights, latest_celestia_height, &presubmit_path)
            .await
        {
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
    let latest_submission =
        match finalize_submission(presubmit_state, celestia_height, postsubmit_path).await {
            Err(error) => {
                error!(%error, "failed to finalize submission; abandoning");
                return Err(error);
            }
            Ok(state) => state,
        };

    state.set_celestia_connected(true);
    state.set_latest_confirmed_celestia_height(celestia_height);

    metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_HEIGHT).absolute(celestia_height);
    metrics::histogram!(crate::metrics_init::CELESTIA_SUBMISSION_LATENCY).record(start.elapsed());

    info!(%celestia_height, "successfully submitted blocks to Celestia");

    Ok(latest_submission)
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
                metrics::counter!(crate::metrics_init::CELESTIA_SUBMISSION_FAILURE_COUNT)
                    .increment(1);

                let state = Arc::clone(&state);
                state.set_celestia_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);

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

#[instrument(skip_all)]
async fn fetch_latest_celestia_height(
    client: HttpClient,
    state: Arc<super::State>,
) -> eyre::Result<()> {
    use celestia_client::celestia_rpc::HeaderClient as _;

    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(5))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    %error,
                    "attempt to fetch latest Celestia height failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let height = tryhard::retry_fn(|| {
        let client = client.clone();
        async move {
            let header = client
                .header_network_head()
                .await
                .wrap_err("failed to fetch network head from Celestia node")?;
            Ok(header.height().value())
        }
    })
    .with_config(retry_config)
    .in_current_span()
    .await
    .wrap_err("retry attempts exhausted; bailing")?;
    state.set_latest_confirmed_celestia_height(height);
    Ok(())
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

/// Currently running conversions of Sequencer blocks to Celestia blobs.
///
/// The conversion result will be returned in the order they are pushed
/// into this queue.
///
/// Note on the implementation: the conversions are done in a block tokio
/// task so that conversions are started immediately without needing extra
/// polling. This means that the only contribution that `FuturesOrdered`
/// provides is ordering the conversion result by the order the blocks are
/// received. This however is desirable because we want to submit sequencer
/// blocks in the order of their heights to Celestia.
struct Conversions {
    // The currently active conversions.
    active: FuturesOrdered<BoxFuture<'static, (SequencerHeight, eyre::Result<Vec<Blob>>)>>,

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

    fn push(&mut self, block: SequencerBlock) {
        let height = block.height();
        let fut = async move {
            let res = tokio::task::spawn_blocking(move || convert(block)).await;
            let res = crate::utils::flatten(res);
            (height, res)
        }
        .boxed();
        self.active.push_back(fut);
    }

    async fn next(&mut self) -> Option<(SequencerHeight, eyre::Result<Vec<Blob>>)> {
        use tokio_stream::StreamExt as _;
        self.active.next().await
    }
}

fn convert(block: SequencerBlock) -> eyre::Result<Vec<Blob>> {
    let mut blobs = Vec::new();
    block
        .try_to_blobs(&mut blobs)
        .wrap_err("failed converting sequencer block to celestia blobs")?;
    Ok(blobs)
}

#[instrument(skip_all)]
async fn initialize_submission<P: AsRef<Path>>(
    sequencer_heights: &[SequencerHeight],
    celestia_height_at_init: u64,
    presubmit_path: P,
) -> eyre::Result<PreSubmissionState> {
    let largest_height = sequencer_heights
        .iter()
        .copied()
        .max()
        .expect("there should be blobs and accompanying sequencer heights to submit");
    let presubmit_state = PreSubmissionState {
        celestia_height_at_init,
        sequencer_height: largest_height.value(),
    };
    debug!(
        state = serde_json::to_string(&presubmit_state).expect("type contains no non-ascii keys"),
        "initializing submission by writing state to file",
    );
    let bytes = serde_json::to_vec(&presubmit_state)
        .wrap_err("failed to serialize presubmission state as JSON")?;
    let mut file = tokio::fs::File::options()
        .write(true)
        .truncate(true)
        .open(presubmit_path)
        .await
        .wrap_err("failed opening file to store presubmission state")?;
    file.write_all(&bytes)
        .await
        .wrap_err("failed to write presubmission state to file")?;
    file.sync_all()
        .await
        .wrap_err("failed fully syncing presubmission state to disk")?;
    Ok(presubmit_state)
}

async fn finalize_submission<P: AsRef<Path>>(
    presubmit_state: PreSubmissionState,
    celestia_height: u64,
    postsubmit_path: P,
) -> eyre::Result<PostSubmissionState> {
    let postsubmit_state = presubmit_state.into_submitted(celestia_height);
    debug!(
        state = serde_json::to_string(&postsubmit_state).expect("type contains no non-ascii keys"),
        "finalizing submission by writing state to file",
    );

    let bytes = serde_json::to_vec(&postsubmit_state)
        .wrap_err("failed to serialize postsubmission state as JSON")?;
    let mut file = tokio::fs::File::options()
        .write(true)
        .truncate(true)
        .open(postsubmit_path)
        .await
        .wrap_err("failed opening file to store postsubmission state")?;
    file.write_all(&bytes)
        .await
        .wrap_err("failed to write postsubmission state to file")?;
    file.sync_all()
        .await
        .wrap_err("failed fully syncing postsubmission state to disk")?;
    Ok(postsubmit_state)
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
