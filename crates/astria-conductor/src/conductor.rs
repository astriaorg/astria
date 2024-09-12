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
    task::JoinHandle,
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
    metrics::Metrics,
    sequencer,
    utils::flatten,
    Config,
};

/// Token to signal whether the conductor should restart or shut down.
pub enum RestartToken {
    Restart,
    Shutdown,
}

pin_project! {
    /// Handle to the conductor, returned by [`Conductor::spawn`].
    pub struct ConductorHandle {
        shutdown: CancellationToken,
        task: Option<JoinHandle<eyre::Result<()>>>,
    }
}

impl ConductorHandle {
    /// Initiates shutdown of the conductor and returns its result.
    ///
    /// # Errors
    /// Returns an error if the conductor exited with an error.
    ///
    /// # Panics
    /// Panics if shutdown is called twice.
    #[instrument(skip_all, err)]
    pub async fn shutdown(&mut self) -> eyre::Result<()> {
        self.shutdown.cancel();
        self.task
            .take()
            .expect("shutdown must not be called twice")
            .await
            .wrap_err("inner conductor task failed")?
    }
}

impl Future for ConductorHandle {
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

/// A wrapper around [`InnerConductorTask`] that manages shutdown and restart of the conductor.
pub struct Conductor {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown: CancellationToken,

    /// Token to signal to inner conductor task to shut down gracefully.
    conductor_inner_shutdown: CancellationToken,

    /// Handle for the inner conductor task.
    handle: ConductorInnerHandle,

    /// Configuration for the conductor, necessary upon a restart.
    cfg: Config,

    /// Metrics used by tasks, necessary upon a restart.
    metrics: &'static Metrics,
}

impl Conductor {
    /// Creates a new `Conductor` from a [`Config`].
    ///
    /// # Errors
    /// Returns an error if [`InnerConductorTask`] could not be created.
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let conductor = InnerConductorTask::new(cfg.clone(), metrics)?;
        let conductor_inner_shutdown = conductor.shutdown.clone();
        let conductor_inner_handle = conductor.spawn();
        Ok(Self {
            shutdown: CancellationToken::new(),
            conductor_inner_shutdown,
            handle: conductor_inner_handle,
            cfg,
            metrics,
        })
    }

    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        loop {
            select! {
                biased;

                () = self.shutdown.cancelled() => {
                    break;
                },

                task_res = &mut self.handle => {
                    match task_res {
                        Ok(Ok(restart_token)) => {
                            match restart_token {
                                RestartToken::Restart => self.restart(),
                                RestartToken::Shutdown => break,
                            }
                        },
                        Ok(Err(err)) => {
                            let error = err.wrap_err("conductor task exited unexpectedly");
                            Err(error)?;
                        },
                        Err(err) => {
                            let error = eyre::ErrReport::from(err).wrap_err("conductor failed during shutdown");
                            Err(error)?;
                        }
                    }
                }
            }
        }
        self.shutdown().await?;
        Ok(())
    }

    /// Creates and spawns a new [`InnerConductorTask`] task with the same configuration, replacing
    /// the previous one. This function should only be called after a graceful shutdown of the
    /// inner conductor task.
    fn restart(&mut self) {
        info!("restarting conductor");
        let new_handle = InnerConductorTask::new(self.cfg.clone(), self.metrics)
            .expect("failed to create new conductor after restart")
            .spawn();
        self.conductor_inner_shutdown = new_handle.shutdown_token.clone();
        self.handle = new_handle;
    }

    /// Initiates shutdown of all conductor tasks from the top down, ignoring a restart signal.
    async fn shutdown(self) -> eyre::Result<()> {
        self.conductor_inner_shutdown.cancel();
        let shutdown_result = self.handle.await?;
        shutdown_result?;
        Ok(())
    }

    #[must_use]
    pub fn spawn(self) -> ConductorHandle {
        let shutdown = self.shutdown.clone();
        let task = tokio::spawn(self.run_until_stopped());
        ConductorHandle {
            shutdown,
            task: Some(task),
        }
    }
}

pin_project! {
    /// A handle returned by [`InnerConductorTask::spawn`].
    pub struct ConductorInnerHandle {
        shutdown_token: CancellationToken,
        task: Option<tokio::task::JoinHandle<eyre::Result<RestartToken>>>,
    }
}

impl Future for ConductorInnerHandle {
    type Output = Result<eyre::Result<RestartToken>, tokio::task::JoinError>;

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

pub struct InnerConductorTask {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown: CancellationToken,

    /// The different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,
}

impl InnerConductorTask {
    const CELESTIA: &'static str = "celestia";
    const EXECUTOR: &'static str = "executor";
    const SEQUENCER: &'static str = "sequencer";

    /// Create a new [`InnerConductorTask`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let mut tasks = JoinMap::new();

        let sequencer_cometbft_client = HttpClient::new(&*cfg.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        let shutdown = CancellationToken::new();

        // Spawn the executor task.
        let executor_handle = {
            let (executor, handle) = executor::Builder {
                mode: cfg.execution_commit_level,
                rollup_address: cfg.execution_rpc_url,
                shutdown: shutdown.clone(),
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
                shutdown: shutdown.clone(),
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
                shutdown: shutdown.clone(),
                metrics,
            }
            .build()
            .wrap_err("failed to build Celestia Reader")?;

            tasks.spawn(Self::CELESTIA, reader.run_until_stopped());
        };

        Ok(Self {
            shutdown,
            tasks,
        })
    }

    /// Runs [`InnerConductorTask`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    async fn run_until_stopped(mut self) -> eyre::Result<RestartToken> {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));
        let mut should_restart = false;

        let exit_reason = select! {
            biased;

            () = self.shutdown.cancelled() => {
                Ok("received shutdown signal")
            }

            Some((name, res)) = self.tasks.join_next() => {
                match flatten(res) {
                    Ok(()) => Err(eyre!("task `{name}` exited unexpectedly")),
                    Err(err) => {
                        if name == Self::EXECUTOR {
                            should_restart = check_for_restart(&err);
                        }
                        Err(err).wrap_err_with(|| "task `{name}` failed")
                    },
                }
            }
        };

        let message = "initiating shutdown";
        report_exit(&exit_reason, message);
        let shutdown_res = self.shutdown().await;

        if should_restart {
            return Ok(RestartToken::Restart);
        }

        if let RestartToken::Restart = shutdown_res {
            return Ok(RestartToken::Restart);
        }
        exit_reason?;
        Ok(RestartToken::Shutdown)
    }

    /// Spawns Conductor on the tokio runtime.
    ///
    /// This calls [`tokio::spawn`] and returns a [`ConductorInnerHandle`] to the
    /// running Conductor task.
    #[must_use]
    pub fn spawn(self) -> ConductorInnerHandle {
        let shutdown_token = self.shutdown.clone();
        let task = tokio::spawn(self.run_until_stopped());
        ConductorInnerHandle {
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
    async fn shutdown(mut self) -> RestartToken {
        self.shutdown.cancel();
        let mut should_restart = false;

        info!("signalled all tasks to shut down; waiting for 25 seconds to exit");

        let shutdown_loop = async {
            while let Some((name, res)) = self.tasks.join_next().await {
                let message = "task shut down";
                match flatten(res) {
                    Ok(()) => info!(name, message),
                    Err(error) => {
                        if name == Self::EXECUTOR {
                            should_restart = check_for_restart(&error);
                        }
                        error!(name, %error, message);
                    }
                }
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

        if should_restart {
            RestartToken::Restart
        } else {
            RestartToken::Shutdown
        }
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
