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
use tendermint_rpc::HttpClient;
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

/// Token to signal whether the conductor should restart or shut down.
pub(super) enum RestartOrShutdown {
    Restart,
    Shutdown,
}

pin_project! {
    /// A handle returned by [`ConductorInner::spawn`].
    pub(super) struct InnerHandle {
        shutdown_token: CancellationToken,
        task: Option<tokio::task::JoinHandle<eyre::Result<RestartOrShutdown>>>,
    }
}

impl Future for InnerHandle {
    type Output = Result<eyre::Result<RestartOrShutdown>, tokio::task::JoinError>;

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
    pub(super) fn new(
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
    async fn run_until_stopped(mut self) -> eyre::Result<RestartOrShutdown> {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));

        let exit_reason = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                Ok("received shutdown signal")
            }

            Some((name, res)) = self.tasks.join_next() => {
                match flatten(res) {
                    Ok(()) => Err(eyre!("task `{name}` exited unexpectedly")),
                    Err(err) => {
                        Err(err).wrap_err_with(|| format!("task `{name}` failed"))
                    },
                }
            }
        };

        let message = "initiating shutdown";
        report_exit(&exit_reason, message);
        self.shutdown(exit_reason).await
    }

    /// Spawns Conductor on the tokio runtime.
    ///
    /// This calls [`tokio::spawn`] and returns a [`InnerHandle`] to the
    /// running Conductor task.
    #[must_use]
    pub(super) fn spawn(self) -> InnerHandle {
        let shutdown_token = self.shutdown_token.clone();
        let task = tokio::spawn(self.run_until_stopped());
        InnerHandle {
            shutdown_token,
            task: Some(task),
        }
    }

    /// Shuts down all tasks.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    #[instrument(skip_all)]
    async fn shutdown(
        mut self,
        exit_reason: eyre::Result<&str>,
    ) -> eyre::Result<RestartOrShutdown> {
        self.shutdown_token.cancel();
        let mut finished_tasks = vec![exit_reason];

        info!("signalled all tasks to shut down; waiting for 25 seconds to exit");

        let shutdown_loop = async {
            while let Some((name, res)) = self.tasks.join_next().await {
                let message = "task shut down";
                let res = match flatten(res) {
                    Ok(()) => {
                        info!(name, message);
                        Ok("task exited successfullly")
                    }
                    Err(error) => {
                        error!(name, %error, message);
                        Err(error).wrap_err_with(|| format!("task `{name}` failed"))
                    }
                };
                finished_tasks.push(res);
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

        finished_tasks.reverse();
        // If any tasks failed and don't warrant a restart, return their error
        for task_res in finished_tasks {
            if let Err(err) = task_res {
                let Some(task_name) = task_failed(&err) else {
                    continue;
                };
                if task_name == "executor" && check_for_restart(&err) {
                    return Ok(RestartOrShutdown::Restart);
                }
                return Err(err);
            }
        }

        Ok(RestartOrShutdown::Shutdown)
    }
}

#[instrument(skip_all)]
fn report_exit(exit_reason: &eyre::Result<&str>, message: &str) {
    match exit_reason {
        Ok(reason) => info!(%reason, message),
        Err(reason) => error!(%reason, message),
    }
}

#[instrument(skip_all)]
fn check_for_restart(err: &eyre::ErrReport) -> bool {
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

fn task_failed(err: &eyre::ErrReport) -> Option<String> {
    let err = err.to_string();
    if err.contains("task `executor` failed") {
        return Some("executor".to_string());
    }
    if err.contains("task `sequencer` failed") {
        return Some("sequencer".to_string());
    }
    if err.contains("task `celestia` failed") {
        return Some("celestia".to_string());
    }
    None
}
