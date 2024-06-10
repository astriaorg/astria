use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
    time::Duration,
};

use astria_core::{
    generated::sequencerblock::v1alpha1::sequencer_service_client::SequencerServiceClient,
    sequencerblock::v1alpha1::SequencerBlock,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    WrapErr as _,
};
use futures::{
    future::{
        BoxFuture,
        Fuse,
        FusedFuture as _,
    },
    FutureExt as _,
};
use sequencer_client::{
    tendermint::block::Height as SequencerHeight,
    tendermint_rpc,
    HttpClient as SequencerClient,
};
use tokio::{
    select,
    sync::{
        mpsc::error::TrySendError,
        watch,
    },
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use tracing::{
    debug,
    error,
    info,
    instrument,
    trace,
    warn,
    Instrument,
    Span,
};

use crate::{
    validator::Validator,
    IncludeRollup,
};

mod builder;
mod celestia_client;
mod read;
mod state;
mod submission;
mod write;

pub(crate) use builder::Builder;
use celestia_client::{
    BuilderError,
    CelestiaClientBuilder,
    CelestiaKeys,
    TrySubmitError,
};
use state::State;
pub(crate) use state::StateSnapshot;

use self::submission::SubmissionState;

pub(crate) struct Relayer {
    /// A token to notify relayer that it should shut down.
    shutdown_token: CancellationToken,

    /// The configured chain ID of the sequencer network.
    sequencer_chain_id: String,

    /// The client used to query the sequencer cometbft endpoint.
    sequencer_cometbft_client: SequencerClient,

    /// The client used to poll the sequencer via the sequencer gRPC API.
    sequencer_grpc_client: SequencerServiceClient<Channel>,

    /// The poll period defines the fixed interval at which the sequencer is polled.
    sequencer_poll_period: Duration,

    /// The gRPC client for submitting sequencer blocks to celestia.
    celestia_client_builder: CelestiaClientBuilder,

    /// The rollups whose data should be included in submissions.
    rollup_filter: IncludeRollup,

    /// A watch channel to track the state of the relayer. Used by the API service.
    state: Arc<State>,

    pre_submit_path: PathBuf,
    post_submit_path: PathBuf,
}

impl Relayer {
    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<state::StateSnapshot> {
        self.state.subscribe()
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// Returns errors if sequencer block fetch or celestia blob submission
    /// failed catastrophically (after `u32::MAX` retries).
    #[instrument(skip_all)]
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let submission_state = read_submission_state(&self.pre_submit_path, &self.post_submit_path)
            .await
            .wrap_err("failed reading submission state from files")?;

        select!(
            () = self.shutdown_token.cancelled() => return Ok(()),
            init_result = confirm_sequencer_chain_id(
                self.sequencer_chain_id.clone(),
                self.sequencer_cometbft_client.clone()
            ) => init_result,
        )?;

        let last_submitted_sequencer_height = submission_state.last_submitted_height();

        let mut latest_height_stream = {
            use sequencer_client::StreamLatestHeight as _;
            self.sequencer_cometbft_client
                .stream_latest_height(self.sequencer_poll_period)
        };

        let (submitter_task, submitter) = spawn_submitter(
            self.celestia_client_builder.clone(),
            self.rollup_filter.clone(),
            self.state.clone(),
            submission_state,
            self.shutdown_token.clone(),
        );

        let mut block_stream = read::BlockStream::builder()
            .block_time(self.sequencer_poll_period)
            .client(self.sequencer_grpc_client.clone())
            .set_last_fetched_height(last_submitted_sequencer_height)
            .state(self.state.clone())
            .build();

        // future to forward a sequencer block to the celestia-submission-task.
        // gets set in the select-loop if the task is at capacity.
        let mut forward_once_free: Fuse<
            BoxFuture<Result<(), tokio::sync::mpsc::error::SendError<SequencerBlock>>>,
        > = Fuse::terminated();

        self.state.set_ready();

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("received shutdown signal");
                    break Ok("shutdown signal received");
                }

                res = &mut forward_once_free, if !forward_once_free.is_terminated() => {
                    // XXX: exiting because submitter only returns an error after u32::MAX
                    // retries, which is practically infinity.
                    if res.is_err() {
                        break Err(eyre!("submitter exited unexpectedly while trying to forward block"));
                    }
                    block_stream.resume();
                    debug!("block stream resumed");
                }

                Some(res) = latest_height_stream.next() => {
                    match res {
                        Ok(height) => {
                            self.state.set_latest_observed_sequencer_height(height.value());
                            debug!(%height, "received latest height from sequencer");
                            block_stream.set_latest_sequencer_height(height);
                        }
                        Err(error) => {
                            metrics::counter!(crate::metrics_init::SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT)
                                .increment(1);
                            self.state.set_sequencer_connected(false);
                            warn!(
                                %error,
                                "failed fetching latest height from sequencer; waiting until next tick",
                            );
                        }
                    }
                }

                Some((height, fetch_result)) = block_stream.next() => {
                    let block = match fetch_result.wrap_err_with(||
                        format!(
                            "relayer ultimately failed fetching sequencer block at height {height}"
                    )) {
                        // XXX: exiting because the fetch in block_stream errors after u32::MAX
                        // retries, which is practically infinity.
                        Err(err) => break Err(err),
                        Ok(block) => block,
                    };
                    self.state.set_latest_fetched_sequencer_height(height.value());
                    if let Err(err) = self.forward_block_for_submission(
                        height,
                        block,
                        &mut block_stream,
                        submitter.clone(),
                        &mut forward_once_free,
                    ).wrap_err("submitter exited unexpectedly while trying to forward block") {
                        // XXX: exiting because there is no logic to restart the blob-submitter task.
                        // With the current implementation of the task it should also never go down
                        // unless it has exhausted all u32::MAX attempts to submit to Celestia and
                        // ultimately failed (after what's practically years of trying...).
                        break Err(err);
                    }
                }
            );
        };

        match &reason {
            Ok(reason) => info!(reason, "starting shutdown"),
            Err(reason) => error!(%reason, "starting shutdown"),
        }

        debug!("waiting for Celestia submission task to exit");
        if let Err(error) = submitter_task.await {
            error!(%error, "Celestia submission task failed while waiting for it to exit before shutdown");
        }

        reason.map(|_| ())
    }

    #[instrument(skip_all, fields(%height))]
    fn forward_block_for_submission(
        &self,
        height: SequencerHeight,
        block: SequencerBlock,
        block_stream: &mut read::BlockStream,
        submitter: write::BlobSubmitterHandle,
        forward: &mut Fuse<
            BoxFuture<Result<(), tokio::sync::mpsc::error::SendError<SequencerBlock>>>,
        >,
    ) -> eyre::Result<()> {
        assert!(
            forward.is_terminated(),
            "block stream must be paused and not yield blocks when the blob submitter is \
             congested and this future is in-flight",
        );

        if let Err(error) = submitter.try_send(block) {
            debug!(
                // Just print the error directly: TrySendError has no cause chain.
                %error,
                "failed forwarding sequencer block to submitter; \
                pausing block stream and scheduling for later submission",
            );
            block_stream.pause();
            debug!("block stream paused");

            match error {
                TrySendError::Full(block) => {
                    *forward = async move { submitter.send(block).await }.boxed().fuse();
                }
                TrySendError::Closed(..) => bail!("blob submitter has shut down unexpectedly"),
            }
        }
        Ok(())
    }
}

#[instrument(skip_all)]
async fn confirm_sequencer_chain_id(
    configured_sequencer_chain_id: String,
    sequencer_cometbft_client: SequencerClient,
) -> eyre::Result<()> {
    let span = Span::current();

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .max_delay(Duration::from_secs(30))
        .exponential_backoff(Duration::from_secs(1))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = %eyre::Report::new(error.clone()),
                    "failed to fetch sequencer chain id; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let received_sequencer_chain_id =
        tryhard::retry_fn(move || fetch_sequencer_chain_id(sequencer_cometbft_client.clone()))
            .with_config(retry_config)
            .in_current_span()
            .await
            .wrap_err("retry attempts exhausted; bailing")?;

    ensure!(
        received_sequencer_chain_id == configured_sequencer_chain_id,
        "configured sequencer chain ID does not match received; configured: \
         `{configured_sequencer_chain_id}`, received: `{received_sequencer_chain_id}`"
    );
    info!(sequencer_chain_id = %configured_sequencer_chain_id, "confirmed sequencer chain id");
    Ok(())
}

async fn fetch_sequencer_chain_id(
    sequencer_cometbft_client: SequencerClient,
) -> Result<String, tendermint_rpc::Error> {
    use sequencer_client::Client as _;

    let response = sequencer_cometbft_client.status().await;
    // trace-level logging, so using Debug format is ok.
    #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
    {
        trace!(?response);
    }
    response.map(|status_response| status_response.node_info.network.to_string())
}

async fn read_submission_state<P1: AsRef<Path>, P2: AsRef<Path>>(
    pre: P1,
    post: P2,
) -> eyre::Result<SubmissionState> {
    const LENIENT_CONSISTENCY_CHECK: bool = true;
    let pre = pre.as_ref().to_path_buf();
    let post = post.as_ref().to_path_buf();
    crate::utils::flatten(
        tokio::task::spawn_blocking(move || {
            SubmissionState::from_paths::<LENIENT_CONSISTENCY_CHECK, _, _>(pre, post)
        })
        .await,
    )
    .wrap_err(
        "failed reading submission state from the configured pre- and post-submit files. Refer to \
         the values documented in `local.env.example` of the astria-sequencer-relayer service",
    )
}

fn spawn_submitter(
    client_builder: CelestiaClientBuilder,
    rollup_filter: IncludeRollup,
    state: Arc<State>,
    submission_state: SubmissionState,
    shutdown_token: CancellationToken,
) -> (JoinHandle<eyre::Result<()>>, write::BlobSubmitterHandle) {
    let (submitter, handle) = write::BlobSubmitter::new(
        client_builder,
        rollup_filter,
        state,
        submission_state,
        shutdown_token,
    );
    (tokio::spawn(submitter.run()), handle)
}

struct ReportValidator<'a>(&'a Validator);

impl<'a> std::fmt::Display for ReportValidator<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0.address))
    }
}
