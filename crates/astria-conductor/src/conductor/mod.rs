mod inner;

use std::{
    future::Future,
    task::ready,
};

use astria_eyre::eyre::{
    self,
    Result,
    WrapErr as _,
};
use inner::{
    Inner,
    RestartOrShutdown,
};
use pin_project_lite::pin_project;
use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::{
    metrics::Metrics,
    Config,
};

pin_project! {
    /// Handle to the conductor, returned by [`Conductor::spawn`].
    pub struct Handle {
        shutdown_token: CancellationToken,
        task: Option<JoinHandle<eyre::Result<()>>>,
    }
}

impl Handle {
    /// Initiates shutdown of the conductor and returns its result.
    ///
    /// # Errors
    /// Returns an error if the conductor exited with an error.
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

impl Future for Handle {
    type Output = eyre::Result<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use futures::future::FutureExt as _;
        let this = self.project();
        let task = this
            .task
            .as_mut()
            .expect("the Conductor handle must not be polled after shutdown");

        let res = ready!(task.poll_unpin(cx));
        std::task::Poll::Ready(crate::utils::flatten(res))
    }
}

/// A wrapper around [`ConductorInner`] that manages shutdown and restart of the conductor.
pub struct Conductor {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    /// Handle for the inner conductor task.
    inner: JoinHandle<eyre::Result<RestartOrShutdown>>,

    /// Configuration for the conductor, necessary upon a restart.
    cfg: Config,

    /// Metrics used by tasks, necessary upon a restart.
    metrics: &'static Metrics,
}

impl Conductor {
    /// Creates a new `Conductor` from a [`Config`].
    ///
    /// # Errors
    /// Returns an error if [`ConductorInner`] could not be created.
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
        let inner = Inner::new(cfg.clone(), metrics, shutdown_token.child_token())?;
        Ok(Self {
            shutdown_token,
            inner: tokio::spawn(inner.run_until_stopped()),
            cfg,
            metrics,
        })
    }

    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        loop {
            let exit_reason = (&mut self.inner).await;
            match self.restart_or_shutdown(exit_reason).await? {
                RestartOrShutdown::Restart => self.restart()?,
                RestartOrShutdown::Shutdown => break Ok(()),
            }
        }
    }

    /// Creates and spawns a new [`ConductorInner`] task with the same configuration, replacing
    /// the previous one. This function should only be called after a graceful shutdown of the
    /// inner conductor task.
    #[instrument(skip_all, err)]
    fn restart(&mut self) -> eyre::Result<()> {
        self.inner = tokio::spawn(
            Inner::new(
                self.cfg.clone(),
                self.metrics,
                self.shutdown_token.child_token(),
            )
            .wrap_err("failed to instantiate Conductor for restart")?
            .run_until_stopped(),
        );
        Ok(())
    }

    /// Reports if conductor will shutdown or restart.
    ///
    /// This method only exists to encapsulate tracing and generate
    /// events for restart, shutdown, or errors.
    #[instrument(skip_all, err, ret(Display))]
    async fn restart_or_shutdown(
        &mut self,
        exit_reason: Result<Result<RestartOrShutdown>, JoinError>,
    ) -> eyre::Result<RestartOrShutdown> {
        match exit_reason {
            Ok(Ok(restart_or_shutdown)) => Ok(restart_or_shutdown),
            Ok(Err(err)) => Err(err.wrap_err("conductor exited with an error")),
            Err(err) => Err(eyre::Report::new(err).wrap_err("conductor panicked")),
        }
    }

    #[must_use]
    pub fn spawn(self) -> Handle {
        let shutdown_token = self.shutdown_token.clone();
        let task = tokio::spawn(self.run_until_stopped());
        Handle {
            shutdown_token,
            task: Some(task),
        }
    }
}
