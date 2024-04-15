use std::time::Duration;

use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use itertools::Itertools as _;
use sequencer_client::HttpClient;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
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
    sequencer,
    utils::flatten,
    Config,
};

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
        let mut tasks = JoinMap::new();

        let sequencer_cometbft_client = HttpClient::new(&*cfg.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        let shutdown = CancellationToken::new();

        // Spawn the executor task.
        let executor_handle = {
            let (executor, handle) = executor::Builder {
                consider_commitment_spread: !cfg.execution_commit_level.is_soft_only(),
                rollup_address: cfg.execution_rpc_url,
                shutdown: shutdown.clone(),
            }
            .build()
            .wrap_err("failed constructing exectur")?;

            tasks.spawn(Self::EXECUTOR, executor.run_until_stopped());
            handle
        };

        if !cfg.execution_commit_level.is_firm_only() {
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

        if !cfg.execution_commit_level.is_soft_only() {
            let reader = celestia::Builder {
                celestia_http_endpoint: cfg.celestia_node_http_url,
                celestia_token: cfg.celestia_bearer_token,
                celestia_block_time: Duration::from_millis(cfg.celestia_block_time_ms),
                executor: executor_handle.clone(),
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                shutdown: shutdown.clone(),
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
    pub async fn run_until_stopped(mut self) {
        info!("conductor is running");

        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
        );

        let exit_reason = select! {
            biased;

            _ = sigterm.recv() => Ok("received SIGTERM"),

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
