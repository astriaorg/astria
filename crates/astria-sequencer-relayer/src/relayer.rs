use std::time::Duration;

use eyre::{
    bail,
    Result,
    WrapErr as _,
};
use futures::stream::TryStreamExt as _;
use humantime::format_duration;
use sequencer_types::SequencerBlockData;
use tendermint_rpc::{
    event::Event,
    query::{
        EventType,
        Query,
    },
    SubscriptionClient,
    WebSocketClient,
};
use tokio::{
    select,
    sync::watch,
    task,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    macros::report_err,
    validator::Validator,
};

pub struct Relayer {
    /// The websocket client for the sequencer, used to create a `NewBlock` subscription.
    sequencer_ws_client: WebSocketClient,

    // The client for submitting sequencer blocks to the data availability layer.
    data_availability: CelestiaClient,

    // Carries the signing key to sign sequencer blocks before they are submitted to the data
    // availability layer.
    validator: Validator,

    // A watch channel to track the state of the relayer. Used by the API service.
    state_tx: watch::Sender<State>,

    // Sequencer blocks that have been received but not yet submitted to the data availability
    // layer (for example, because a submit RPC was currently in flight) .
    queued_blocks: Vec<SequencerBlockData>,

    // A collection of workers to convert a raw cometbft/tendermint block response to
    // the sequencer block data type.
    conversion_workers: task::JoinSet<eyre::Result<Option<SequencerBlockData>>>,

    // Task to submit blocks to the data availability layer. If this is set it means that
    // an RPC is currently in flight and new blocks are queued up. They will be submitted
    // once this task finishes.
    submission_task: Option<task::JoinHandle<eyre::Result<u64>>>,
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
    pub async fn new(cfg: &crate::config::Config) -> Result<Self> {
        let (sequencer_ws_client, driver) = WebSocketClient::new(&*cfg.sequencer_endpoint)
            .await
            .wrap_err("failed to create sequencer client")?;
        tokio::spawn(async move { driver.run().await }); // TODO move to self.run()

        let validator = Validator::from_path(&cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let data_availability = CelestiaClient::builder()
            .endpoint(&cfg.celestia_endpoint)
            .bearer_token(&cfg.celestia_bearer_token)
            .gas_limit(cfg.gas_limit)
            .build()
            .wrap_err("failed to create data availability client")?;

        let (state_tx, _) = watch::channel(State::default());

        Ok(Self {
            sequencer_ws_client,
            data_availability,
            validator,
            state_tx,
            queued_blocks: Vec::new(),
            conversion_workers: task::JoinSet::new(),
            submission_task: None,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state_tx.subscribe()
    }

    /// Handle an [`Event`] returned from the sequencer websocket subscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not a `NewBlock` event or if the event
    /// does not contain a block.
    fn handle_new_block_event(&mut self, event: Event) -> eyre::Result<()> {
        let tendermint_rpc::event::EventData::NewBlock {
            block: maybe_block,
            ..
        } = event.data
        else {
            bail!("sequencer websocket subscription returned unexpected event");
        };
        let Some(block) = maybe_block else {
            bail!("sequencer websocket subscription returned event without block");
        };

        info!(
            height = %block.header.height,
            tx.count = block.data.len(),
            "received block from sequencer"
        );
        let current_height = self.state_tx.borrow().current_sequencer_height;
        let validator = self.validator.clone();

        // Start the costly conversion; note that the current height at
        // the time of receipt matters. The internal state might have advanced
        // past the height recorded in the block while it was converting, but
        // that's ok.
        self.conversion_workers.spawn_blocking(move || {
            convert_tendermint_block_to_sequencer_block_data(block, current_height, validator)
        });
        Ok(())
    }

    /// Handle the result
    #[instrument(skip_all)]
    fn handle_conversion_completed(
        &mut self,
        join_result: Result<Result<Option<SequencerBlockData>>, task::JoinError>,
    ) {
        // First check if the join task panicked
        let conversion_result = match join_result {
            Ok(conversion_result) => conversion_result,
            // Report if the task failed, i.e. panicked
            Err(e) => {
                // TODO: inject the correct tracing span
                report_err!(e, "conversion task failed");
                return;
            }
        };
        // Then handle the actual result of the computation
        match conversion_result {
            // Collect successfully converted sequencer responses
            Ok(Some(sequencer_block_data)) => {
                // Update the internal state if the block was admitted
                let height = sequencer_block_data.header().height.value();
                self.state_tx.send_if_modified(|state| {
                    if Some(height) > state.current_sequencer_height {
                        state.current_sequencer_height = Some(height);
                        return true;
                    }
                    false
                });
                // Store the converted data
                self.queued_blocks.push(sequencer_block_data);
            }
            // Ignore sequencer responses that were filtered out
            Ok(None) => (),
            // Report if the conversion failed
            // TODO: inject the correct tracing span
            Err(e) => report_err!(
                e,
                "failed converting sequencer block response to block data"
            ),
        }
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
                // TODO: inject the correct tracing span
                report_err!(e, "submission task failed");
                return;
            }
        };
        // Then report update the internal state or report if submission failed
        match submission_result {
            Ok(height) => self.state_tx.send_modify(|state| {
                state.current_data_availability_height.replace(height);
            }),
            // TODO: add more context to this error, maybe inject a span?
            Err(e) => report_err!(e, "submitting blocks to data availability layer failed"),
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
        let client = self.data_availability.clone();
        debug!("attempting to connect to data availability layer",);
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        let height = (|| {
            let client = client.clone();
            async move { client.get_latest_height().await }
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

        debug!("attempting to connect to sequencer layer");
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        (|| {
            let client = self.sequencer_ws_client.clone();
            async move { client.abci_info().await }
        })
        .retry(&backoff)
        .await
        .wrap_err("failed to retrieve latest height from sequencer layer after several retries")?;
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
    /// - if waiting for DA or the sequencer fails
    /// - if subscribing to the `NewBlock` event on the sequencer fails
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

        let mut stream = self
            .sequencer_ws_client
            .subscribe(Query::from(EventType::NewBlock))
            .await
            .wrap_err("failed to subscribe to NewBlock event")?;

        loop {
            select!(
                // Receive new blocks from the sequencer
                res = stream.try_next() => {
                    match res {
                        Ok(maybe_event) => {
                            let Some(event) = maybe_event else {
                                error!(
                                    "sequencer websocket subscription returned None"
                                );
                                break;
                            };
                            self.handle_new_block_event(event).wrap_err("failed to handle new block event")?;
                        }
                        Err(e) => report_err!(e, "failed getting latest block from sequencer"),
                    }
                }

                // Distribute and store converted/admitted blocks
                Some(res) = self.conversion_workers.join_next() => self.handle_conversion_completed(res),

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
                self.submission_task = Some(task::spawn(submit_blocks_to_data_availability_layer(
                    client,
                    self.queued_blocks.clone(),
                    self.validator.clone(),
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
            self.conversion_workers.abort_all();
            if let Some(task) = self.submission_task.as_mut() {
                task.abort()
            }
            Ok(())
        }
    }
}

#[instrument(skip_all)]
fn convert_tendermint_block_to_sequencer_block_data(
    block: tendermint::Block,
    current_height: Option<u64>,
    validator: Validator,
) -> eyre::Result<Option<SequencerBlockData>> {
    if Some(block.header.height.value()) <= current_height {
        debug!(
            "sequencer block response contained height at or below the current height tracked in \
             relayer"
        );
        return Ok(None);
    }
    if block.header.proposer_address != validator.address {
        debug!("proposer recorded in sequencer block response does not match internal validator");
        return Ok(None);
    }
    let sequencer_block_data = SequencerBlockData::from_tendermint_block(block)
        .wrap_err("failed converting sequencer block response to sequencer block data")?;
    Ok(Some(sequencer_block_data))
}

#[instrument(skip_all)]
async fn submit_blocks_to_data_availability_layer(
    client: CelestiaClient,
    sequencer_block_data: Vec<SequencerBlockData>,
    validator: Validator,
) -> eyre::Result<u64> {
    info!(
        num_blocks = sequencer_block_data.len(),
        "submitting collected sequencer blocks to data availability layer",
    );
    let rsp = client
        .submit_all_blocks(sequencer_block_data, &validator.signing_key)
        .await?;
    Ok(rsp.height)
}
