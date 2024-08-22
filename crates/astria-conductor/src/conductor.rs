use std::{
    future::Future,
    sync::OnceLock,
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

pin_project! {
    /// A handle returned by [`Conductor::spawn`].
    pub struct Handle {
        shutdown_token: CancellationToken,
        task: Option<tokio::task::JoinHandle<()>>,
    }
}

impl Handle {
    /// Sends a signal to the conductor task to shut down.
    ///
    /// # Errors
    /// Returns an error if the Conductor task panics during shutdown.
    ///
    /// # Panics
    /// Panics if called twice.
    pub async fn shutdown(&mut self) -> Result<(), tokio::task::JoinError> {
        self.shutdown_token.cancel();
        let task = self.task.take().expect("shutdown must not be called twice");
        task.await
    }
}

impl Future for Handle {
    type Output = Result<(), tokio::task::JoinError>;

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

pub struct Conductor {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown: CancellationToken,

    /// The different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,
}

impl Conductor {
    const CELESTIA: &'static str = "celestia";
    const EXECUTOR: &'static str = "executor";
    const SEQUENCER: &'static str = "sequencer";

    /// Create a new [`Conductor`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    pub fn new(cfg: Config) -> eyre::Result<Self> {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        let metrics = METRICS.get_or_init(Metrics::new);

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

    /// Runs [`Conductor`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    #[instrument(skip_all)]
    async fn run_until_stopped(mut self) {
        info!("conductor is running");

        let exit_reason = select! {
            biased;

            () = self.shutdown.cancelled() => {
                Ok("received shutdown signal")
            }

            Some((name, res)) = self.tasks.join_next() => {
                match flatten(res) {
                    Ok(()) => Err(eyre!("task `{name}` exited unexpectedly")),
                    Err(err) => Err(err).wrap_err_with(|| "task `{name}` failed"),
                }
            }
        };

        let message = "initiating shutdown";
        match exit_reason {
            Ok(reason) => info!(reason, message),
            Err(reason) => error!(%reason, message),
        }
        self.shutdown().await;
    }

    /// Spawns Conductor on the tokio runtime.
    ///
    /// This calls [`tokio::spawn`] and returns a [`Handle`] to the
    /// running Conductor task, allowing to explicitly shut it down with
    /// [`Handle::shutdown`].
    #[must_use]
    pub fn spawn(self) -> Handle {
        let shutdown_token = self.shutdown.clone();
        let task = tokio::spawn(self.run_until_stopped());
        Handle {
            shutdown_token,
            task: Some(task),
        }
    }

    /// Shuts down all tasks.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    async fn shutdown(mut self) {
        self.shutdown.cancel();

        info!("signalled all tasks to shut down; waiting for 25 seconds to exit");

        let shutdown_loop = async {
            while let Some((name, res)) = self.tasks.join_next().await {
                let message = "task shut down";
                match flatten(res) {
                    Ok(()) => info!(name, message),
                    Err(error) => error!(name, %error, message),
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
    }
}
