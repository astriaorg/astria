mod inner;

use std::future::Future;

use astria_eyre::eyre;
use inner::{
    ConductorInner,
    InnerHandle,
    RestartOrShutdown,
};
use pin_project_lite::pin_project;
use tokio::task::{
    JoinError,
    JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    instrument,
};

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
            .expect("the Conductor handle must not be polled after shutdown");
        task.poll_unpin(cx)
    }
}

/// A wrapper around [`ConductorInner`] that manages shutdown and restart of the conductor.
pub struct Conductor {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    /// Handle for the inner conductor task.
    inner: InnerHandle,

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
        let conductor_inner_handle =
            ConductorInner::spawn(cfg.clone(), metrics, shutdown_token.child_token())?;
        Ok(Self {
            shutdown_token,
            inner: conductor_inner_handle,
            cfg,
            metrics,
        })
    }

    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        loop {
            let exit_reason = (&mut self.inner).await;
            self.shutdown_or_restart(exit_reason).await?;
            if self.shutdown_token.is_cancelled() {
                break;
            }
        }
        Ok(())
    }

    /// Creates and spawns a new [`ConductorInner`] task with the same configuration, replacing
    /// the previous one. This function should only be called after a graceful shutdown of the
    /// inner conductor task.
    fn restart(&mut self) {
        info!("restarting conductor");
        let new_handle = ConductorInner::spawn(
            self.cfg.clone(),
            self.metrics,
            self.shutdown_token.child_token(),
        )
        .expect("failed to create new conductor after restart");
        self.inner = new_handle;
    }

    /// Initiates either a restart or a shutdown of all conductor tasks.
    #[instrument(skip_all, err)]
    async fn shutdown_or_restart(
        &mut self,
        exit_reason: Result<RestartOrShutdown, JoinError>,
    ) -> eyre::Result<&'static str> {
        match exit_reason {
            Ok(restart_or_shutdown) => match restart_or_shutdown {
                RestartOrShutdown::Restart => {
                    self.restart();
                    return Ok("restarting");
                }
                RestartOrShutdown::Shutdown => Ok("conductor exiting"),
            },
            Err(err) => Err(eyre::ErrReport::from(err).wrap_err("conductor failed")),
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
