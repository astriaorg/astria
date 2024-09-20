use std::{
    future::Future,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use itertools::Itertools as _;
use pin_project_lite::pin_project;
use sequencer_client::HttpClient;
use tokio::{
    select,
    time::timeout,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    info,
    info_span,
    instrument,
    warn,
};

use crate::{
    celestia,
    executor,
    sequencer,
    utils::flatten,
    Config,
    Metrics,
};

/// Exit value of the inner conductor impl to signal to the outer task whether to restart or
/// shutdown
pub(super) enum RestartOrShutdown {
    Restart,
    Shutdown,
}

enum ExitReason {
    ShutdownSignal,
    TaskFailed {
        name: &'static str,
        error: eyre::ErrReport,
    },
}

pin_project! {
    /// A handle returned by [`ConductorInner::spawn`].
    pub(super) struct InnerHandle {
        shutdown_token: CancellationToken,
        task: Option<tokio::task::JoinHandle<RestartOrShutdown>>,
    }
}

impl Future for InnerHandle {
    type Output = Result<RestartOrShutdown, tokio::task::JoinError>;

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

pub(super) struct ConductorInner {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    /// The different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,
}

impl ConductorInner {
    const CELESTIA: &'static str = "celestia";
    const EXECUTOR: &'static str = "executor";
    const SEQUENCER: &'static str = "sequencer";

    /// Create a new [`ConductorInner`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    fn new(
        cfg: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let mut tasks = JoinMap::new();

        let sequencer_cometbft_client = HttpClient::new(&*cfg.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        // Spawn the executor task.
        let executor_handle = {
            let (executor, handle) = executor::Builder {
                mode: cfg.execution_commit_level,
                rollup_address: cfg.execution_rpc_url,
                shutdown: shutdown_token.clone(),
                metrics,
            }
            .build()
            .wrap_err("failed constructing executor")?;

            tasks.spawn(Self::EXECUTOR, executor.run_until_stopped());
            handle
        };

        if cfg.execution_commit_level.is_with_soft() {
            let sequencer_grpc_client =
                sequencer::SequencerGrpcClient::new(&cfg.sequencer_grpc_url)
                    .wrap_err("failed constructing grpc client for Sequencer")?;

            // The `sync_start_block_height` represents the height of the next
            // sequencer block that can be executed on top of the rollup state.
            // This value is derived by the Executor.
            let sequencer_reader = sequencer::Builder {
                sequencer_grpc_client,
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                sequencer_block_time: Duration::from_millis(cfg.sequencer_block_time_ms),
                shutdown: shutdown_token.clone(),
                executor: executor_handle.clone(),
            }
            .build();
            tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
        }

        if cfg.execution_commit_level.is_with_firm() {
            let celestia_token = if cfg.no_celestia_auth {
                None
            } else {
                Some(cfg.celestia_bearer_token)
            };

            let reader = celestia::Builder {
                celestia_http_endpoint: cfg.celestia_node_http_url,
                celestia_token,
                celestia_block_time: Duration::from_millis(cfg.celestia_block_time_ms),
                executor: executor_handle.clone(),
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                sequencer_requests_per_second: cfg.sequencer_requests_per_second,
                shutdown: shutdown_token.clone(),
                metrics,
            }
            .build()
            .wrap_err("failed to build Celestia Reader")?;

            tasks.spawn(Self::CELESTIA, reader.run_until_stopped());
        };

        Ok(Self {
            shutdown_token,
            tasks,
        })
    }

    /// Runs [`ConductorInner`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    async fn run_until_stopped(mut self) -> RestartOrShutdown {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));

        let exit_reason = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                ExitReason::ShutdownSignal
            },

            Some((name, res)) = self.tasks.join_next() => {
                match flatten(res) {
                    Ok(()) => ExitReason::TaskFailed{name, error: eyre!("task `{name}` exited unexpectedly")},
                    Err(err) => ExitReason::TaskFailed{name, error: err.wrap_err(format!("task `{name}` failed"))},
                }
            }
        };

        let message = "initiating shutdown";
        report_exit(&exit_reason, message);
        self.shutdown(exit_reason).await
    }

    /// Creates and spawns a Conductor on the tokio runtime.
    ///
    /// This calls [`tokio::spawn`] and returns a [`InnerHandle`] to the
    /// running Conductor task.
    pub(super) fn spawn(
        cfg: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<InnerHandle> {
        let conductor = Self::new(cfg, metrics, shutdown_token)?;
        let shutdown_token = conductor.shutdown_token.clone();
        let task = tokio::spawn(conductor.run_until_stopped());
        Ok(InnerHandle {
            shutdown_token,
            task: Some(task),
        })
    }

    /// Shuts down all tasks.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    #[instrument(skip_all)]
    async fn shutdown(mut self, exit_reason: ExitReason) -> RestartOrShutdown {
        self.shutdown_token.cancel();
        let mut restart_or_shutdown = RestartOrShutdown::Shutdown;

        match &exit_reason {
            ExitReason::ShutdownSignal => {
                info!("received shutdown signal, skipping check for restart");
            }
            ExitReason::TaskFailed {
                name,
                error,
            } => {
                if check_for_restart(name, error) {
                    restart_or_shutdown = RestartOrShutdown::Restart;
                }
            }
        }

        info!("signalled all tasks to shut down; waiting for 25 seconds to exit");

        let shutdown_loop = async {
            while let Some((name, res)) = self.tasks.join_next().await {
                let message = "task shut down";
                match flatten(res) {
                    Ok(()) => {
                        info!(name, message);
                    }
                    Err(error) => {
                        if check_for_restart(name, &error)
                            && !matches!(exit_reason, ExitReason::ShutdownSignal)
                        {
                            restart_or_shutdown = RestartOrShutdown::Restart;
                        }
                        error!(name, %error, message);
                    }
                };
            }
        };

        if timeout(Duration::from_secs(25), shutdown_loop)
            .await
            .is_err()
        {
            let tasks = self.tasks.keys().join(", ");
            warn!(
                tasks = format_args!("[{tasks}]"),
                "aborting all tasks that have not yet shut down",
            );
            self.tasks.abort_all();
        } else {
            info!("all tasks shut down regularly");
        }
        info!("shutting down");

        restart_or_shutdown
    }
}

#[instrument(skip_all)]
fn report_exit(exit_reason: &ExitReason, message: &str) {
    match exit_reason {
        ExitReason::ShutdownSignal => info!(reason = "received shutdown signal", message),
        ExitReason::TaskFailed {
            name: task,
            error: reason,
        } => error!(%reason, %task, message),
    }
}

#[instrument(skip_all)]
fn check_for_restart(name: &str, err: &eyre::ErrReport) -> bool {
    if name != ConductorInner::EXECUTOR {
        return false;
    }
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
mod test {
    use astria_eyre::eyre::WrapErr as _;

    #[test]
    fn check_for_restart_ok() {
        let tonic_error: Result<&str, tonic::Status> =
            Err(tonic::Status::new(tonic::Code::PermissionDenied, "error"));
        let err = tonic_error.wrap_err("wrapper_1");
        let err = err.wrap_err("wrapper_2");
        let err = err.wrap_err("wrapper_3");
        assert!(super::check_for_restart("executor", &err.unwrap_err()));
    }
}
