use std::{
    fmt::{
        Display,
        Write,
    },
    sync::Arc,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    bail,
    Report,
    WrapErr as _,
};
use celestia_client::{
    celestia_types::Blob,
    jsonrpsee::http_client::HttpClient as CelestiaClient,
};
use futures::{
    future::FusedFuture as _,
    FutureExt as _,
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
    error,
    info,
    instrument,
    warn,
    Span,
};

use crate::{
    metrics_init,
    validator::Validator,
};

pub(crate) struct Relayer {
    /// The actual client used to poll the sequencer.
    sequencer: HttpClient,

    /// The poll period defines the fixed interval at which the sequencer is polled.
    sequencer_poll_period: Duration,

    // The http client for submitting sequencer blocks to celestia.
    data_availability: CelestiaClient,

    // If this is set, only relay blocks to DA which are proposed by the same validator key.
    validator: Option<Validator>,

    // A watch channel to track the state of the relayer. Used by the API service.
    state_tx: watch::Sender<State>,

    // Sequencer blocks that have been received but not yet submitted to the data availability
    // layer (for example, because a submit RPC was currently in flight) .
    queued_blocks: Vec<SequencerBlock>,

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
        let mut submission = futures::future::Fuse::terminated();

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

                        Ok(Err(error)) => {
                            warn!(%error, "failed getting the latest block from sequencer");
                        }
                        Err(error) => {
                            warn!(%error, "task panicked getting the latest block from sequencer");
                        }
                    }
                }

                res = &mut submission, if !submission.is_terminated() => {
                    match res {
                        Err(error) => {
                            metrics::counter!(metrics_init::CELESTIA_SUBMISSION_FAILURE_COUNT).increment(1);
                            error!(%error, "failed submitting blocks to celestia");
                        }
                        Ok(height) => self.state_tx.send_modify(|state| {
                            state.current_data_availability_height.replace(height);
                        }),
                    }
                }
            );
            // Try to submit new blocks
            //
            // This will immediately and eagerly try to submit to the data availability
            // layer if no submission is in flight.
            if !self.queued_blocks.is_empty() && submission.is_terminated() {
                let client = self.data_availability.clone();
                submission = submit_sequencer_blocks(client, self.queued_blocks.clone())
                    .boxed()
                    .fuse();
                self.queued_blocks.clear();
            }
        }
    }
}

#[instrument(skip_all, fields(heights = %ReportBlockHeights(&sequencer_blocks)))]
async fn submit_sequencer_blocks(
    client: CelestiaClient,
    sequencer_blocks: Vec<SequencerBlock>,
) -> eyre::Result<u64> {
    use celestia_client::submission::ToBlobs as _;

    let span = Span::current();
    let conversion_task = tokio::task::spawn_blocking(move || {
        let mut blobs = Vec::new();
        for block in sequencer_blocks {
            let height = block.height();
            if let Err(error) = block.try_to_blobs(&mut blobs) {
                let error = Report::new(error);
                error!(
                    parent: &span,
                    %error,
                    %height,
                    "failed converting sequencer block to celestia blobs",
                );
            }
        }
        blobs
    });
    let blobs = match conversion_task.await {
        Err(error) => {
            // Reuse the message so it's not repeated in both the error field and in
            // the event message. Slightly verbose bust nicer logging.
            let message = "task panicked converting sequencer blocks to Celestia blobs";
            let error = Report::new(error);
            error!(%error, message);
            return Err(error.wrap_err(message));
        }
        Ok(blobs) => blobs,
    };
    let height = match submit_blobs(client, blobs).await {
        Err(error) => {
            let message = "failed submitting blobs to Celestia";
            error!(%error, message);
            return Err(error.wrap_err(message));
        }
        Ok(height) => height,
    };
    metrics::counter!(metrics_init::CELESTIA_SUBMISSION_HEIGHT).absolute(height);
    info!(celestia_height = %height, "successfully submitted blocks to Celestia");
    Ok(height)
}

#[instrument(skip_all)]
async fn submit_blobs(client: CelestiaClient, blobs: Vec<Blob>) -> eyre::Result<u64> {
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
            move |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
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
    .await
    .wrap_err("retry attempts exhausted; bailing")?;
    Ok(height)
}

struct ReportBlockHeights<'a>(&'a [SequencerBlock]);

impl<'a> Display for ReportBlockHeights<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        let mut blocks = self.0.iter();
        if let Some(height) = blocks.next().map(SequencerBlock::height) {
            let mut buf = itoa::Buffer::new();
            f.write_str(buf.format(height.value()))?;
        }
        while let Some(height) = blocks.next().map(SequencerBlock::height) {
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
    use astria_core::sequencer::v1alpha1::{
        test_utils::ConfigureCometBftBlock,
        SequencerBlock,
    };

    use super::ReportBlockHeights;

    fn make_sequencer_block(height: u32) -> SequencerBlock {
        let cometbft_block = ConfigureCometBftBlock {
            height,
            ..ConfigureCometBftBlock::default()
        }
        .make();
        SequencerBlock::try_from_cometbft(cometbft_block).unwrap()
    }

    #[track_caller]
    fn assert_block_height_formatting(heights: &[u32], expected: &str) {
        let blocks: Vec<_> = heights.iter().copied().map(make_sequencer_block).collect();
        let actual = ReportBlockHeights(&blocks).to_string();
        assert_eq!(&actual, expected);
    }

    #[test]
    fn reported_block_heights_formatting() {
        assert_block_height_formatting(&[], "[]");
        assert_block_height_formatting(&[1], "[1]");
        assert_block_height_formatting(&[4, 2, 1], "[4, 2, 1]");
    }
}
