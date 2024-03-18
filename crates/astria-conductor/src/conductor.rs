use std::{
    collections::HashMap,
    error::Error as StdError,
    rc::Rc,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::celestia_types::nmt::Namespace;
use sequencer_client::HttpClient;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::oneshot,
    task::{
        spawn_local,
        LocalSet,
    },
    time::timeout,
};
use tokio_util::task::JoinMap;
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    celestia,
    executor::Executor,
    sequencer,
    Config,
};

pub struct Conductor {
    /// Channels to the long-running tasks to shut them down gracefully
    shutdown_channels: HashMap<&'static str, oneshot::Sender<()>>,

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
    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        let mut tasks = JoinMap::new();
        let mut shutdown_channels = HashMap::new();

        let sequencer_cometbft_client = HttpClient::new(&*cfg.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        // Spawn the executor task.
        let executor_handle = {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            let (executor, handle) = Executor::builder()
                .set_consider_commitment_spread(!cfg.execution_commit_level.is_soft_only())
                .rollup_address(&cfg.execution_rpc_url)
                .wrap_err("failed to assign rollup node address")?
                .shutdown(shutdown_rx)
                .build();

            tasks.spawn(Self::EXECUTOR, executor.run_until_stopped());
            shutdown_channels.insert(Self::EXECUTOR, shutdown_tx);
            handle
        };

        if !cfg.execution_commit_level.is_firm_only() {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            let sequencer_grpc_client =
                sequencer::SequencerGrpcClient::new(&cfg.sequencer_grpc_url)
                    .wrap_err("failed constructing grpc client for Sequencer")?;

            // The `sync_start_block_height` represents the height of the next
            // sequencer block that can be executed on top of the rollup state.
            // This value is derived by the Executor.
            let sequencer_reader = sequencer::Reader::new(
                sequencer_grpc_client,
                sequencer_cometbft_client.clone(),
                Duration::from_millis(cfg.sequencer_block_time_ms),
                shutdown_rx,
                executor_handle.clone(),
            );
            tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
            shutdown_channels.insert(Self::SEQUENCER, shutdown_tx);
        }

        if !cfg.execution_commit_level.is_soft_only() {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            shutdown_channels.insert(Self::CELESTIA, shutdown_tx);

            // Sequencer namespace is defined by the chain id of attached sequencer node
            // which can be fetched from any block header.
            let sequencer_namespace = get_sequencer_namespace(sequencer_cometbft_client.clone())
                .await
                .wrap_err("failed to get sequencer namespace")?;

            let reader = celestia::Reader::builder()
                .celestia_http_endpoint(&cfg.celestia_node_http_url)
                .celestia_websocket_endpoint(&cfg.celestia_node_websocket_url)
                .celestia_token(&cfg.celestia_bearer_token)
                .executor(executor_handle.clone())
                .sequencer_cometbft_client(sequencer_cometbft_client.clone())
                .sequencer_namespace(sequencer_namespace)
                .shutdown(shutdown_rx)
                .build();

            tasks.spawn(Self::CELESTIA, reader.run_until_stopped());
        };

        Ok(Self {
            shutdown_channels,
            tasks,
        })
    }

    /// Runs [`Conductor`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    pub async fn run_until_stopped(mut self) {
        enum ExitReason {
            Sigterm,
            TaskExited {
                name: &'static str,
            },
            TaskErrored {
                name: &'static str,
                error: eyre::Report,
            },
            TaskPanicked {
                name: &'static str,
                error: tokio::task::JoinError,
            },
        }
        use ExitReason::{
            Sigterm,
            TaskErrored,
            TaskExited,
            TaskPanicked,
        };
        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
        );

        let exit_reason = select! {
            biased;

            _ = sigterm.recv() => Sigterm,

            Some((name, res)) = self.tasks.join_next() => {
                match res {
                    Ok(Ok(())) => TaskExited { name, },
                    Ok(Err(error)) => TaskErrored { name, error},
                    Err(error) => TaskPanicked { name, error },
                }
            }
        };

        match exit_reason {
            Sigterm => info!(reason = "received SIGTERM", "shutting down"),
            TaskExited {
                name,
            } => error!(
                task.name = name,
                reason = "task exited unexpectedly",
                "shutting down"
            ),
            TaskErrored {
                name,
                error,
            } => error!(
                task.name = name,
                reason = "task exited with error",
                %error,
                "shutting down"
            ),
            TaskPanicked {
                name,
                error,
            } => error!(
                task.name = name,
                reason = "task panicked",
                error = &error as &dyn StdError,
                "shutting down",
            ),
        }
        self.shutdown().await;
    }

    async fn shutdown(self) {
        info!("sending shutdown command to all tasks");
        for (_, channel) in self.shutdown_channels {
            let _ = channel.send(());
        }

        info!("waiting 5 seconds for all tasks to shut down");
        // put the tasks into an Rc to make them 'static so they can run on a local set
        let mut tasks = Rc::new(self.tasks);
        let local_set = LocalSet::new();
        local_set
            .run_until(async {
                let mut tasks = tasks.clone();
                let _ = timeout(
                    Duration::from_secs(5),
                    spawn_local(async move {
                        while let Some((name, res)) = Rc::get_mut(&mut tasks)
                            .expect(
                                "only one Rc to the conductor tasks should exist; this is a bug",
                            )
                            .join_next()
                            .await
                        {
                            match res {
                                Ok(Ok(())) => info!(task.name = name, "task exited normally"),
                                Ok(Err(e)) => {
                                    let error: &(dyn std::error::Error + 'static) = e.as_ref();
                                    error!(task.name = name, error, "task exited with error");
                                }
                                Err(e) => {
                                    let error = &e as &(dyn std::error::Error + 'static);
                                    error!(task.name = name, error, "task failed");
                                }
                            }
                        }
                    }),
                )
                .await;
            })
            .await;

        if !tasks.is_empty() {
            warn!(
                number = tasks.len(),
                "aborting tasks that haven't shutdown yet"
            );
            Rc::get_mut(&mut tasks)
                .expect("only one Rc to the conductor tasks should exist; this is a bug")
                .shutdown()
                .await;
        }
    }
}

/// Get the sequencer namespace from the latest sequencer block.
async fn get_sequencer_namespace(client: HttpClient) -> eyre::Result<Namespace> {
    use sequencer_client::SequencerClientExt as _;

    let retry_config = tryhard::RetryFutureConfig::new(10)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32,
             next_delay: Option<Duration>,
             error: &sequencer_client::extension_trait::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to grab sequencer block failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let block = tryhard::retry_fn(|| client.latest_sequencer_block())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get block from sequencer after 10 attempts")?;

    Ok(
        celestia_client::celestia_namespace_v0_from_cometbft_chain_id(
            block.header().chain_id().as_str(),
        ),
    )
}
