use std::future::Future;

use astria_eyre::eyre::{
    self,
};
use pin_project_lite::pin_project;
use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{
    Config,
    Metrics,
};

mod inner;

pin_project! {
    /// Handle to the [`Auctioneer`] service, returned by [`Auctioneer::spawn`].
    pub struct Auctioneer {
        shutdown_token: CancellationToken,
        task: Option<JoinHandle<eyre::Result<()>>>,
    }
}

impl Auctioneer {
    /// Creates an [`Auctioneer`] service and runs it, returning a handle to the taks and shutdown
    /// token.
    ///
    /// # Errors
    /// Returns an error if the Auctioneer cannot be initialized.
    #[must_use]
    pub fn spawn(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
        let inner = inner::Auctioneer::new(cfg, metrics, shutdown_token.child_token())?;
        let task = tokio::spawn(inner.run());

        Ok(Self {
            shutdown_token,
            task: Some(task),
        })
    }

    /// Initiates shutdown of the Auctioneer and returns its result.
    ///
    /// # Errors
    /// Returns an error if the Auctioneer exited with an error.
    ///
    /// # Panics
    /// Panics if shutdown is called twice.
    #[instrument(skip_all, err)]
    pub async fn shutdown(&mut self) -> Result<eyre::Result<()>, JoinError> {
        self.shutdown_token.cancel();
        self.task
            .take()
            .expect("shutdown must not be called twice")
            .await
    }
}

impl Future for Auctioneer {
    type Output = Result<eyre::Result<()>, tokio::task::JoinError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use futures::future::FutureExt as _;

        let this = self.project();
        let task = this
            .task
            .as_mut()
            .expect("the Auctioneer handle must not be polled after shutdown");
        task.poll_unpin(cx)
    }
}
