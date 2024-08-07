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
use celestia_rpc::HeaderClient;
use itertools::Itertools as _;
use jsonrpsee::http_client;
use pin_project_lite::pin_project;
use sequencer_client::HttpClient;
use tendermint::Genesis;
use tendermint_rpc::{
    client,
    Client,
};
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
    celestia::{
        self,
        builder::create_celestia_client,
    },
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
        pub task: Option<tokio::task::JoinHandle<eyre::Result<()>>>
    }
}

/// Clients used for initialization checks, formatted as options to allow for soft only and/or firm
/// only configurations.
struct Clients {
    sequencer_client: Option<client::HttpClient>,
    celestia_client: Option<http_client::HttpClient>,
}

/// Errors that can occur during chain ID checks (initialization)
#[derive(Debug, thiserror::Error)]
pub enum InitializationError {
    #[error("failed to get sequencer chain ID")]
    GetSequencerChainID(#[source] sequencer_client::tendermint_rpc::Error),
    #[error("failed to get Celestia chain ID")]
    GetCelestiaChainID(#[source] jsonrpsee::core::Error),
    #[error("expected Celestia chain ID `{expected}`, received `{actual}`")]
    WrongCelestiaChainID { expected: String, actual: String },
    #[error("expected sequencer chain ID `{expected}`, received `{actual}`")]
    WrongSequencerChainID { expected: String, actual: String },
}

trait GetChainID {
    async fn get_chain_id(&self) -> Result<String, InitializationError>;
}

/// Get chain ID implementation for sequencer
impl GetChainID for HttpClient {
    async fn get_chain_id(&self) -> Result<String, InitializationError> {
        let sequencer_genesis: Genesis = self
            .genesis()
            .await
            .map_err(InitializationError::GetSequencerChainID)?;
        Ok(sequencer_genesis.chain_id.to_string())
    }
}

/// Get chain ID implementation for Celestia
impl GetChainID for jsonrpsee::http_client::HttpClient {
    async fn get_chain_id(&self) -> Result<String, InitializationError> {
        let celestia_response = self
            .header_network_head()
            .await
            .map_err(InitializationError::GetCelestiaChainID)?;
        let chain_id = celestia_response.chain_id().to_string();
        Ok(chain_id)
    }
}

impl Handle {
    /// Sends a signal to the conductor task to shut down.
    ///
    /// # Errors
    /// Returns an error if the Conductor task panics during shutdown.
    pub async fn shutdown(&mut self) -> Result<eyre::Result<()>, tokio::task::JoinError> {
        self.shutdown_token.cancel();
        if let Some(task) = self.task.take() {
            task.await
        } else {
            info!("Conductor handle was already shut down");
            Ok(Ok(()))
        }
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

pub struct Conductor {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown: CancellationToken,

    /// The different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,

    /// The configuration information for performing initialization checks
    cfg: Config,

    /// The sequencer and celestia clients for initialization checks
    clients: Clients,
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
        let mut clients = Clients {
            sequencer_client: None,
            celestia_client: None,
        };

        let sequencer_cometbft_client = HttpClient::new(&*cfg.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        let shutdown = CancellationToken::new();

        // Spawn the executor task.
        let executor_handle = {
            let (executor, handle) = executor::Builder {
                mode: cfg.execution_commit_level,
                rollup_address: cfg.execution_rpc_url.clone(),
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
            clients.sequencer_client = Some(sequencer_cometbft_client.clone());
            tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
        }

        if cfg.execution_commit_level.is_with_firm() {
            let reader = celestia::Builder {
                celestia_http_endpoint: cfg.celestia_node_http_url.clone(),
                celestia_token: cfg.celestia_bearer_token.clone(),
                celestia_block_time: Duration::from_millis(cfg.celestia_block_time_ms),
                executor: executor_handle.clone(),
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                sequencer_requests_per_second: cfg.sequencer_requests_per_second,
                shutdown: shutdown.clone(),
                metrics,
            }
            .build()
            .wrap_err("failed to build Celestia Reader")?;
            clients.celestia_client = Some(create_celestia_client(
                cfg.celestia_node_http_url.clone(),
                cfg.celestia_bearer_token.clone().as_str(),
            )?);
            tasks.spawn(Self::CELESTIA, reader.run_until_stopped());
        };

        Ok(Self {
            shutdown,
            tasks,
            cfg,
            clients,
        })
    }

    /// Runs [`Conductor`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    #[instrument(skip_all)]
    async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info!("conductor is running");

        select! {
            biased;

            () = self.shutdown.cancelled() => {
                info!("received shutdown signal during initialization");
                self.shutdown().await;
                return Ok(());
            }

            res = self.init() => {
                res.wrap_err("initialization checks failed")?;
            }
        };

        select! {
            biased;

            () = self.shutdown.cancelled() => {
                info!("received shutdown signal");
                self.shutdown().await;
                return Ok(());
            }

            Some((name, res)) = self.tasks.join_next() => {
                self.shutdown().await;
                match flatten(res) {
                    Ok(()) => return Err(eyre!("task `{name}` exited unexpectedly")),
                    Err(err) => return Err(err).wrap_err_with(|| "task `{name}` failed"),
                }
            }
        };
    }

    /// Performs initialization checks prior to running the conductor.
    async fn init(&self) -> Result<(), InitializationError> {
        self.ensure_chain_ids_are_correct().await?;
        Ok(())
    }

    /// Ensures that provided chain IDs match sequencer and/or celestia chain IDs.
    async fn ensure_chain_ids_are_correct(&self) -> Result<(), InitializationError> {
        let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
            .exponential_backoff(Duration::from_millis(100))
            .max_delay(Duration::from_secs(20))
            .on_retry(
                |attempt: u32, next_delay: Option<Duration>, error: &InitializationError| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        attempt,
                        wait_duration,
                        error = error as &dyn std::error::Error,
                        "attempt to fetch sequencer genesis and/or celestia network head info; \
                         retrying after backoff",
                    );
                    futures::future::ready(())
                },
            );
        if let Some(celestia_client) = &self.clients.celestia_client {
            let celestia_chain_id = tryhard::retry_fn(|| celestia_client.get_chain_id())
                .with_config(retry_config)
                .await?;
            if celestia_chain_id != self.cfg.celestia_chain_id {
                return Err(InitializationError::WrongCelestiaChainID {
                    expected: self.cfg.celestia_chain_id.clone(),
                    actual: celestia_chain_id,
                });
            }
        }
        if let Some(sequencer_client) = &self.clients.sequencer_client {
            let sequencer_chain_id = tryhard::retry_fn(|| sequencer_client.get_chain_id())
                .with_config(retry_config)
                .await?;
            if sequencer_chain_id != self.cfg.sequencer_chain_id {
                return Err(InitializationError::WrongSequencerChainID {
                    expected: self.cfg.sequencer_chain_id.clone(),
                    actual: sequencer_chain_id,
                });
            }
        }
        Ok(())
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
