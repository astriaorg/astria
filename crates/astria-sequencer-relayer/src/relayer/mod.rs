use std::time::Duration;

use astria_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use celestia_client::jsonrpsee::http_client::HttpClient as CelestiaClient;
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
    tendermint_rpc::Client as _,
    HttpClient,
    HttpClient as SequencerClient,
    SequencerBlock,
};
use tokio::{
    pin,
    select,
    sync::{
        mpsc::error::TrySendError,
        watch,
    },
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tracing::{
    debug,
    error,
    field::DisplayValue,
    info,
    instrument,
    warn,
};

use crate::validator::Validator;

mod read;
mod write;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub(crate) data_availability_connected: bool,
    pub(crate) sequencer_connected: bool,
    pub(crate) current_sequencer_height: Option<u64>,
    pub(crate) current_data_availability_height: Option<u64>,
}

impl State {
    pub fn is_ready(&self) -> bool {
        self.data_availability_connected && self.sequencer_connected
    }
}

pub(crate) struct Relayer {
    /// The actual client used to poll the sequencer.
    sequencer: HttpClient,

    /// The poll period defines the fixed interval at which the sequencer is polled.
    sequencer_poll_period: Duration,

    // The http client for submitting sequencer blocks to celestia.
    celestia: CelestiaClient,

    // If this is set, only relay blocks to DA which are proposed by the same validator key.
    validator: Option<Validator>,

    // A watch channel to track the state of the relayer. Used by the API service.
    state_tx: watch::Sender<State>,
}

impl Relayer {
    /// Instantiates a `Relayer`.
    ///
    /// # Errors
    ///
    /// Returns one of the following errors:
    /// + failed to read the validator keys from the path in cfg;
    /// + failed to construct a client to the data availability layer (unless `cfg.disable_writing`
    ///   is set).
    pub(crate) async fn new(cfg: &crate::config::Config) -> eyre::Result<Self> {
        let sequencer = HttpClient::new(&*cfg.sequencer_endpoint)
            .wrap_err("failed to create sequencer client")?;

        let validator = match (
            &cfg.relay_only_validator_key_blocks,
            &cfg.validator_key_file,
        ) {
            (true, Some(file)) => Some(
                Validator::from_path(file).wrap_err("failed to get validator info from file")?,
            ),
            (true, None) => {
                bail!("validator key file must be set if `disable_relay_all` is set")
            }
            (false, _) => None, // could also say that the file was unnecessarily set, but it's ok
        };

        let celestia_client::celestia_rpc::Client::Http(celestia) =
            celestia_client::celestia_rpc::Client::new(
                &cfg.celestia_endpoint,
                Some(&cfg.celestia_bearer_token),
            )
            .await
            .wrap_err("failed constructing celestia http client")?
        else {
            bail!("expected to get a celestia HTTP client, but got a websocket client");
        };

        let (state_tx, _) = watch::channel(State::default());

        Ok(Self {
            sequencer,
            sequencer_poll_period: Duration::from_millis(cfg.block_time),
            celestia,
            validator,
            state_tx,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state_tx.subscribe()
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// Returns errors if sequencer block fetch or celestia blob submission
    /// failed catastrophically (after `u32::MAX` retries).
    #[instrument(skip_all)]
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let latest_height_stream =
            make_latest_height_stream(self.sequencer.clone(), self.sequencer_poll_period);
        pin!(latest_height_stream);

        let (submitter_task, submitter) = spawn_submitter(self.celestia.clone());

        let mut block_stream = read::BlockStream::new(self.sequencer.clone());
        block_stream.set_block_time(self.sequencer_poll_period);

        let mut forward_once_free: Fuse<
            BoxFuture<Result<(), tokio::sync::mpsc::error::SendError<SequencerBlock>>>,
        > = Fuse::terminated();

        let reason = loop {
            select!(
                biased;

                res = &mut forward_once_free, if !forward_once_free.is_terminated() => {
                    block_stream.resume();
                    // XXX: exiting because submitter only returns an error after u32::MAX
                    // retries, which is practically infinity.
                    if res.is_err() {
                        let err = res.wrap_err(
                            "submitter exited unexpectly while trying to forward block"
                        );
                        break err;
                    }
                }

                Some(res) = latest_height_stream.next() => {
                    match res {
                        Ok(height) => {
                            debug!(%height, "received latest height from sequencer");
                            block_stream.set_latest_sequencer_height(height);
                        }
                        Err(error) => warn!(
                            %error,
                            "failed fetching latest height from sequencer; waiting until next tick",
                        ),
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
                    if let Err(err) = self.forward_block_for_submission(
                        height,
                        block,
                        &mut block_stream,
                        submitter.clone(),
                        &mut forward_once_free,
                    ).wrap_err("submitter exited unexpectly while trying to forward block") {
                        // XXX: exiting because submitter only returns an error after u32::MAX
                        // retries, which is practically infinity.
                        break Err(err);
                    }
                }
            );
        };

        submitter_task.abort();
        if let Err(error) = crate::utils::flatten(submitter_task.await) {
            error!(%error, "Celestia blob submission task failed");
        }
        reason
    }

    fn report_validator(&self) -> Option<DisplayValue<ReportValidator<'_>>> {
        self.validator
            .as_ref()
            .map(ReportValidator)
            .map(tracing::field::display)
    }

    fn block_does_not_match_validator(&self, block: &SequencerBlock) -> bool {
        self.validator
            .as_ref()
            .is_some_and(|val| val.address != block.header().proposer_address)
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

        if self.block_does_not_match_validator(&block) {
            info!(
                address.validator = self.report_validator(),
                address.block_proposer = %block.header().proposer_address,
                "block proposer does not match internal validator; dropping",
            );
            return Ok(());
        }
        if let Err(error) = submitter.try_send(block) {
            debug!(
                // Just print the error directly: TrySendError has no cause chain.
                %error,
                "failed forwarding sequencer block to submitter; \
                pausing block stream and scheduling for later submission",
            );
            block_stream.pause();
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

fn spawn_submitter(
    client: CelestiaClient,
) -> (JoinHandle<eyre::Result<()>>, write::BlobSubmitterHandle) {
    let (submitter, handle) = write::BlobSubmitter::new(client);
    (tokio::spawn(submitter.run()), handle)
}

fn make_latest_height_stream(
    client: SequencerClient,
    poll_period: Duration,
) -> impl StreamExt<Item = eyre::Result<SequencerHeight>> {
    use tokio::time::MissedTickBehavior;
    use tokio_stream::wrappers::IntervalStream;
    let mut interval = tokio::time::interval(poll_period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    IntervalStream::new(interval).then(move |_| {
        let client = client.clone();
        async move {
            let commit = client
                .latest_commit()
                .await
                .wrap_err("failed getting latest commit")?;
            Ok(commit.signed_header.header.height)
        }
    })
}

struct ReportValidator<'a>(&'a Validator);
impl<'a> std::fmt::Display for ReportValidator<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0.address))
    }
}
