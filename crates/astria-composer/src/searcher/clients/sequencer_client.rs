use std::time::Duration;

use color_eyre::eyre::{self, Context};
use humantime::format_duration;
use tendermint::abci;
use tracing::{debug, instrument, warn};

/// A thin wrapper around [`sequencer_client::Client`] to add timeouts.
///
/// Currently only provides a timeout for `abci_info`.
#[derive(Clone)]
pub(crate) struct SequencerClient {
    pub(crate) inner: sequencer_client::HttpClient,
}

impl SequencerClient {
    #[instrument]
    pub(crate) fn new(url: &str) -> eyre::Result<Self> {
        let inner = sequencer_client::HttpClient::new(url)
            .wrap_err("failed to construct sequencer client")?;
        Ok(Self { inner })
    }

    /// Wrapper around [`Client::abci_info`] with a 1s timeout.
    pub(crate) async fn abci_info(self) -> eyre::Result<abci::response::Info> {
        use sequencer_client::Client as _;
        tokio::time::timeout(Duration::from_secs(1), self.inner.abci_info())
            .await
            .wrap_err("request timed out")?
            .wrap_err("RPC returned with error")
    }

    /// Wait until a connection to the sequencer is established.
    ///
    /// This function tries to establish a connection to the sequencer by
    /// querying its `abci_info` RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    ///
    /// An error is returned if calling the sequencer failed for `n_retries + 1` times.
    #[instrument(skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    pub(crate) async fn wait_for_sequencer(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> eyre::Result<()> {
        use backon::{ExponentialBuilder, Retryable as _};

        debug!("attempting to connect to sequencer",);
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        (|| {
            let client = self.clone();
            async move { client.abci_info().await }
        })
        .retry(&backoff)
        .notify(|err, dur| warn!(error.msg = %err, retry_in = %format_duration(dur), "failed getting abci info; retrying"))
        .await
        .wrap_err(
            "failed to retrieve abci info from sequencer after several retries",
        )?;

        Ok(())
    }
}
