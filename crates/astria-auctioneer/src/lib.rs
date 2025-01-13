//! TODO: Add a description

use std::{
    future::Future,
    task::Poll,
};

mod auctioneer;
mod bid;
mod block;
mod build_info;
pub mod config;
pub(crate) mod metrics;
mod rollup_channel;
mod sequencer_channel;
mod sequencer_key;
mod streaming_utils;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
pub use build_info::BUILD_INFO;
pub use config::Config;
pub use metrics::Metrics;
pub use telemetry;
use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

/// The [`Auctioneer`] service returned by [`Auctioneer::spawn`].
pub struct Auctioneer {
    shutdown_token: CancellationToken,
    task: Option<JoinHandle<eyre::Result<()>>>,
}

impl Auctioneer {
    /// Spawns the [`Auctioneer`] service.
    ///
    /// # Errors
    /// Returns an error if the Auctioneer cannot be initialized.
    pub fn spawn(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
        let inner = auctioneer::Auctioneer::new(cfg, metrics, shutdown_token.child_token())?;
        let task = tokio::spawn(inner.run());

        Ok(Self {
            shutdown_token,
            task: Some(task),
        })
    }

    /// Shuts down Auctioneer, in turn waiting for its components to shut down.
    ///
    /// # Errors
    /// Returns an error if an error occured during shutdown.
    ///
    /// # Panics
    /// Panics if called twice.
    #[instrument(skip_all, err)]
    pub async fn shutdown(&mut self) -> eyre::Result<()> {
        self.shutdown_token.cancel();
        flatten_join_result(
            self.task
                .take()
                .expect("shutdown must not be called twice")
                .await,
        )
    }
}

impl Future for Auctioneer {
    type Output = eyre::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        use futures::future::FutureExt as _;

        let task = self
            .task
            .as_mut()
            .expect("auctioneer must not be polled after shutdown");
        task.poll_unpin(cx).map(flatten_join_result)
    }
}

fn flatten_join_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
