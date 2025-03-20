use std::time::Duration;

use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use tokio::{
    select,
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    info_span,
    instrument,
    warn,
};

use crate::{
    executor,
    Config,
    Metrics,
};

/// Exit value of the inner conductor impl to signal to the outer task whether to restart or
/// shutdown
#[derive(Debug)]
pub(super) enum RestartOrShutdown {
    Restart,
    Shutdown,
}

impl std::fmt::Display for RestartOrShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            RestartOrShutdown::Restart => "restarting",
            RestartOrShutdown::Shutdown => "shutting down",
        };
        f.write_str(msg)
    }
}

struct ShutdownSignalReceived;

/// The business logic of Conductur.
pub(super) struct Inner {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    executor: Option<JoinHandle<eyre::Result<()>>>,
}

impl Inner {
    /// Create a new [`Inner`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    pub(super) fn new(
        config: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let executor = executor::Builder {
            config,
            shutdown: shutdown_token.clone(),
            metrics,
        }
        .build()
        .wrap_err("failed constructing executor")?;

        Ok(Self {
            shutdown_token,
            executor: Some(tokio::spawn(executor.run_until_stopped())),
        })
    }

    /// Runs [`Inner`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<RestartOrShutdown> {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));

        let exit_reason = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                Ok(ShutdownSignalReceived)
            },

            res = self.executor.as_mut().expect("task must always be set at this point") => {
                // XXX: must Option::take the JoinHandle to avoid polling it in the shutdown logic.
                self.executor.take();
                match res {
                    Ok(Ok(())) => Err(eyre!("executor exited unexpectedly")),
                    Ok(Err(err)) => Err(err.wrap_err("executor exited with error")),
                    Err(err) => Err(Report::new(err).wrap_err("executor panicked")),
                }
            }
        };

        self.restart_or_shutdown(exit_reason).await
    }

    /// Shuts down all tasks.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    #[instrument(skip_all, err, ret(Display))]
    async fn restart_or_shutdown(
        mut self,
        exit_reason: eyre::Result<ShutdownSignalReceived>,
    ) -> eyre::Result<RestartOrShutdown> {
        self.shutdown_token.cancel();
        let restart_or_shutdown = match exit_reason {
            Ok(ShutdownSignalReceived) => Ok(RestartOrShutdown::Shutdown),
            Err(error) => {
                error!(%error, "executor failed; checking error chain if conductor should be restarted");
                if check_for_restart(&error) {
                    Ok(RestartOrShutdown::Restart)
                } else {
                    Err(error)
                }
            }
        };

        if let Some(mut executor) = self.executor.take() {
            let wait_until_timeout = Duration::from_secs(25);
            if timeout(wait_until_timeout, &mut executor).await.is_err() {
                warn!(
                    "waited `{}` for executor start to respond to shutdown signal; aborting",
                    telemetry::display::format_duration(wait_until_timeout)
                );
                executor.abort();
            } else {
                info!("executor shut down regularly");
            }
        }

        restart_or_shutdown
    }
}

#[instrument(skip_all)]
fn check_for_restart(err: &eyre::Report) -> bool {
    let mut current = Some(err.as_ref() as &dyn std::error::Error);
    while let Some(err) = current {
        if let Some(status) = err.downcast_ref::<tonic::Status>() {
            if status.code() == tonic::Code::PermissionDenied {
                return true;
            }
        }
        current = err.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use astria_eyre::eyre::WrapErr as _;

    #[test]
    fn check_for_restart_ok() {
        let tonic_error: Result<&str, tonic::Status> =
            Err(tonic::Status::new(tonic::Code::PermissionDenied, "error"));
        let err = tonic_error.wrap_err("wrapper_1");
        let err = err.wrap_err("wrapper_2");
        let err = err.wrap_err("wrapper_3");
        assert!(super::check_for_restart(&err.unwrap_err()));
    }
}
