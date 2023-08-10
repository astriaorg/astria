use std::time::Duration;

use async_trait::async_trait;
use humantime::format_duration;
use tracing::{debug, info, instrument, warn};
use color_eyre::eyre::{self, Context};
use tokio::sync::mpsc as tokio_mpsc;

use ethers::providers::{
    Middleware, Provider as EthersProvider, ProviderError as EthersProviderError, StreamExt, Ws,
};

use super::{streaming_client::StreamingClient, RollupChainId, RollupTxExt};

/// A thin wrapper around [`Provider<Ws>`] to add timeouts.
pub(crate) struct EthProvider {
    inner: EthersProvider<Ws>,
}

// TODO(https://github.com/astriaorg/astria/issues/216): add timeouts for
// `subscribe_full_pendings_txs` (more complex because it's a stream).
impl EthProvider {
    pub(crate) async fn connect(url: &str) -> Result<Self, EthersProviderError> {
        let inner = EthersProvider::connect(url).await?;
        Ok(Self { inner })
    }

    /// Wrapper around [`Provider::get_net_version`] with a 1s timeout.
    async fn get_net_version(&self) -> Result<String, eyre::Error> {
        tokio::time::timeout(Duration::from_secs(1), self.inner.get_net_version())
            .await
            .wrap_err("request timed out")?
            .wrap_err("RPC returned with error")
    }

    /// Wait until a connection to eth is established.
    ///
    /// This function tries to establish a connection to eth by
    /// querying its net_version RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    ///
    /// An error is returned if calling eth failed after `n_retries + 1` times.
    #[instrument(skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    pub(crate) async fn setup(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> Result<(), eyre::Error> {
        use backon::{ExponentialBuilder, Retryable as _};

        debug!("attempting to connect to eth");
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);

        let version = (|| {
            let client = self.inner.clone();
                // NOTE: This is using `get_net_version` because that's what ethers' Middleware is
                // implementing. Maybe the `net_listening` RPC would be better, but ethers
                // does not have that.
                async move { client.get_net_version().await }
            })
            .retry(&backoff)
            .notify(|err, dur| warn!(error.msg = %err, retry_in = %format_duration(dur), "failed issuing RPC; retrying"))
            .await
            .wrap_err(
                "failed to retrieve net version from eth after seferal retries",
            )?;
        info!(version, rpc = "net_version", "RPC was successful");
        Ok(())
    }
}

#[async_trait]
impl StreamingClient for EthProvider {
    type Error = eyre::Error;

    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<tokio_mpsc::UnboundedReceiver<RollupTxExt>, Self::Error> {
        self.inner.start_stream(chain_id).await
    }
}