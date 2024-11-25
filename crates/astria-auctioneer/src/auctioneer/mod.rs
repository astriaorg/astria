use std::{
    future::Future,
    task::Poll,
};

use astria_eyre::eyre::{
    self,
};
use inner::Inner;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{
    flatten_join_result,
    Config,
    Metrics,
};

mod inner;

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
        let inner = Inner::new(cfg, metrics, shutdown_token.child_token())?;
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
