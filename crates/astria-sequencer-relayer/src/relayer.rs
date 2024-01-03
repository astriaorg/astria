use std::time::Duration;

use eyre::{
    bail,
    WrapErr as _,
};
use humantime::format_duration;
use sequencer_client::{
    HttpClient,
    SequencerBlock,
    SequencerClientExt as _,
};
use tokio::{
    select,
    sync::watch,
    task,
    time::interval,
};
use tracing::{
    debug,
    info,
    instrument,
    warn,
};

use crate::validator::Validator;

pub(crate) struct Relayer {
    /// The actual client used to poll the sequencer.
    sequencer: HttpClient,

    /// The poll period defines the fixed interval at which the sequencer is polled.
    sequencer_poll_period: Duration,

    // The http client for submitting sequencer blocks to celestia.
    data_availability: celestia_client::jsonrpsee::http_client::HttpClient,

    // If this is set, only relay blocks to DA which are proposed by the same validator key.
    validator: Option<Validator>,

    // A watch channel to track the state of the relayer. Used by the API service.
    state_tx: watch::Sender<State>,

    // Sequencer blocks that have been received but not yet submitted to the data availability
    // layer (for example, because a submit RPC was currently in flight) .
    queued_blocks: Vec<SequencerBlock>,

    // Task to submit blocks to the data availability layer. If this is set it means that
    // an RPC is currently in flight and new blocks are queued up. They will be submitted
    // once this task finishes.
    submission_task: Option<task::JoinHandle<eyre::Result<u64>>>,

    // Task to query the sequencer for new blocks. A new request will be sent once this
    // task returns.
    sequencer_task: Option<task::JoinHandle<eyre::Result<SequencerBlock>>>,
}

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
                eyre::bail!("validator key file must be set if `disable_relay_all` is set")
            }
            (false, _) => None, // could also say that the file was unnecessarily set, but it's ok
        };

        let celestia_client::celestia_rpc::Client::Http(data_availability) =
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
            data_availability,
            validator,
            state_tx,
            queued_blocks: Vec::new(),
            submission_task: None,
            sequencer_task: None,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state_tx.subscribe()
    }

    #[instrument(skip_all)]
    fn handle_sequencer_tick(&mut self) {
        if self.sequencer_task.is_some() {
            debug!("task polling sequencer is currently in flight; not scheduling a new task");
            return;
        }
        let client = self.sequencer.clone();
        let timeout = self.sequencer_poll_period.checked_mul(2).expect(
            "the sequencer block time should never be set to a value so high that multiplying it \
             by 2 causes it to overflow",
        );
        self.sequencer_task = Some(tokio::spawn(async move {
            let block = tokio::time::timeout(timeout, client.latest_sequencer_block())
                .await
                .wrap_err("timed out getting latest block from sequencer")??;
            Ok(block)
        }));
    }

    #[instrument(skip_all)]
    fn handle_submission_completed(
        &mut self,
        join_result: Result<eyre::Result<u64>, task::JoinError>,
    ) {
        self.submission_task = None;
        // First check if the join task panicked
        let submission_result = match join_result {
            Ok(submission_result) => submission_result,
            // Report if the task failed, i.e. panicked
            Err(e) => {
                warn!(
                    error = &e as &dyn std::error::Error,
                    "submission task failed",
                );
                return;
            }
        };
        // Then report update the internal state or report if submission failed
        match submission_result {
            Ok(height) => self.state_tx.send_modify(|state| {
                debug!(
                    celestia_height=%height,
                    "successfully submitted blocks to data availability layer"
                );
                state.current_data_availability_height.replace(height);
            }),
            Err(e) => warn!(
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                "failed submitting blocks to celestia",
            ),
        }
    }

    /// Wait until a connection to the data availability layer is established.
    ///
    /// This function tries to retrieve the latest height from Celestia.
    /// If it fails, it retries for another `n_retries` times with exponential
    /// backoff.
    ///
    /// # Errors
    /// An error is returned if calling the data availabilty failed for a total
    /// of `n_retries + 1` times.
    #[instrument(name = "Relayer::wait_for_data_availability", skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    async fn wait_for_data_availability_layer(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> eyre::Result<()> {
        use backon::{
            ExponentialBuilder,
            Retryable as _,
        };
        use celestia_client::celestia_rpc::HeaderClient as _;
        let client = self.data_availability.clone();
        debug!("attempting to connect to data availability layer",);
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        let height = (|| {
            let client = client.clone();
            async move {
                let header = client
                    .header_network_head()
                    .await
                    .wrap_err("failed fetching network head")?;
                Ok::<u64, eyre::Report>(header.header.height.value())
            }
        })
        .retry(&backoff)
        .await
        .wrap_err(
            "failed to retrieve latest height from data availability layer after several retries",
        )?;
        self.state_tx.send_modify(|state| {
            state.data_availability_connected = true;
            state.current_data_availability_height.replace(height);
        });
        Ok(())
    }

    /// Wait until a connection to the sequencer is established.
    ///
    /// This function tries to establish a connection to the sequencer by
    /// querying its abci_info RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    /// An error is returned if calling the data availabilty failed for a total
    /// of `n_retries + 1` times.
    #[instrument(name = "Relayer::wait_for_sequencer", skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    async fn wait_for_sequencer(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> eyre::Result<()> {
        use backon::{
            ExponentialBuilder,
            Retryable as _,
        };
        use tendermint_rpc::Client as _;

        debug!("attempting to connect to sequencer",);
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        (|| {
            let client = self.sequencer.clone();
            async move { client.abci_info().await }
        })
        .retry(&backoff)
        .await
        .wrap_err(
            "failed to retrieve latest height from data availability layer after several retries",
        )?;
        self.state_tx.send_modify(|state| {
            // ABCI Info also contains information about the last block, but we
            // purposely don't record it in the state because we want to process
            // it through `get_latest_block`.
            state.sequencer_connected = true;
        });
        Ok(())
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// `Relayer::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    #[instrument(name = "Relayer::run", skip_all)]
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let wait_for_da = self.wait_for_data_availability_layer(5, Duration::from_secs(5), 2.0);
        let wait_for_seq = self.wait_for_sequencer(5, Duration::from_secs(5), 2.0);
        match tokio::try_join!(wait_for_da, wait_for_seq) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start relayer"),
        }
        self.wait_for_sequencer(5, Duration::from_secs(5), 2.0)
            .await
            .wrap_err("failed establishing connection to the sequencer")?;

        let mut sequencer_interval = interval(self.sequencer_poll_period);

        loop {
            select!(
                // Query sequencer for the latest block if no task is in flight
                _ = sequencer_interval.tick() => self.handle_sequencer_tick(),

                // Handle the sequencer response by converting it
                //
                // NOTE: + wrapping the task in an async block makes this lazy;
                //       + `unwrap`ping can't fail because this branch is disabled if `None`
                res = async { self.sequencer_task.as_mut().unwrap().await }, if self.sequencer_task.is_some() => {
                    self.sequencer_task = None;
                    match res {
                        Ok(Ok(block)) if
                            self.validator
                                .as_ref()
                                .is_some_and(|v| v.address != block.header().proposer_address) =>
                        {
                            debug!("proposer of sequencer block does not match internal validator; ignoring");
                        }
                        Ok(Ok(block)) => {
                            // TODO(https://github.com/astriaorg/astria/issues/616): test that only new sequencer
                            // heights are relayed.
                            let height = block.header().height.value();
                            let is_new_block = self.state_tx.send_if_modified(|state| {
                                let is_new_height = Some(height) > state.current_sequencer_height;
                                if is_new_height {
                                    state.current_sequencer_height = Some(height);
                                }
                                is_new_height
                            });
                            if is_new_block {
                                self.queued_blocks.push(block);
                            }
                        }

                        Ok(Err(e)) => {
                            let error: &dyn std::error::Error = e.as_ref();
                            warn!(error, "failed getting the latest block from sequencer");
                        }
                        Err(e) => {
                            warn!(error = &e as &dyn std::error::Error, "task panicked getting the latest block from sequencer");
                        }
                    }
                }

                // Record the current height of the data availability layer if a submission
                // was in flight.
                //
                // NOTE: + wrapping the task in an async block makes this lazy;
                //       + `unwrap`ping can't fail because this branch is disabled if `None`
                res = async { self.submission_task.as_mut().unwrap().await }, if self.submission_task.is_some() => {
                    self.handle_submission_completed(res);
                }
            );
            // Try to submit new blocks
            //
            // This will immediately and eagerly try to submit to the data availability
            // layer if no submission is in flight.
            if !self.queued_blocks.is_empty() && self.submission_task.is_none() {
                let client = self.data_availability.clone();
                self.submission_task = Some(task::spawn(submit_blocks_to_celestia(
                    client,
                    self.queued_blocks.clone(),
                )));
                self.queued_blocks.clear();
            }
        }
        // FIXME(https://github.com/astriaorg/astria/issues/357):
        // Currently relayer's event loop never stops so this code cannot be reached.
        // This should be fixed by shutting it down when receiving a SIGKILL or something
        // like that.
        #[allow(unreachable_code)]
        {
            if let Some(task) = self.submission_task.as_mut() {
                task.abort();
            }
            Ok(())
        }
    }
}

#[instrument(skip_all)]
async fn submit_blocks_to_celestia(
    client: celestia_client::jsonrpsee::http_client::HttpClient,
    sequencer_blocks: Vec<SequencerBlock>,
) -> eyre::Result<u64> {
    use celestia_client::{
        celestia_types::blob::SubmitOptions,
        CelestiaClientExt as _,
    };

    info!(
        num_blocks = sequencer_blocks.len(),
        "submitting collected sequencer blocks to data availability layer",
    );

    let height = client
        .submit_sequencer_blocks(
            sequencer_blocks,
            SubmitOptions {
                fee: None,
                gas_limit: None,
            },
        )
        .await
        .wrap_err("failed submitting sequencer blocks to celestia")?;
    Ok(height)
}
