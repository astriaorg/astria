use std::{
    net::SocketAddr,
    sync::{
        Arc,
        OnceLock,
    },
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    select,
    sync::oneshot,
    task::{
        JoinError,
        JoinHandle,
    },
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
};

pub(crate) use self::state::StateSnapshot;
use self::{
    ethereum::watcher,
    state::State,
    submitter::Submitter,
};
use crate::{
    api,
    config::Config,
    metrics::Metrics,
};

mod batch;
mod ethereum;
mod startup;
mod state;
mod submitter;

pub struct BridgeWithdrawer {
    // Token to signal all subtasks to shut down gracefully.
    shutdown_token: CancellationToken,
    api_server: api::ApiServer,
    submitter: Submitter,
    ethereum_watcher: watcher::Watcher,
    startup: startup::Startup,
    state: Arc<State>,
}

impl BridgeWithdrawer {
    /// Instantiates a new `Service`.
    ///
    /// # Errors
    ///
    /// - If the provided `api_addr` string cannot be parsed as a socket address.
    pub fn new(cfg: Config) -> eyre::Result<(Self, ShutdownHandle)> {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        let metrics = METRICS.get_or_init(Metrics::new);

        let shutdown_handle = ShutdownHandle::new();
        let Config {
            api_addr,
            sequencer_cometbft_endpoint,
            sequencer_chain_id,
            sequencer_key_path,
            sequencer_address_prefix,
            fee_asset_denomination,
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            rollup_asset_denomination,
            sequencer_bridge_address,
            sequencer_grpc_endpoint,
            ..
        } = cfg;

        let state = Arc::new(State::new());

        let sequencer_bridge_address = sequencer_bridge_address
            .parse()
            .wrap_err("failed to parse sequencer bridge address")?;

        // make startup object
        let startup = startup::Builder {
            shutdown_token: shutdown_handle.token(),
            state: state.clone(),
            sequencer_chain_id,
            sequencer_cometbft_endpoint: sequencer_cometbft_endpoint.clone(),
            sequencer_bridge_address,
            sequencer_grpc_endpoint: sequencer_grpc_endpoint.clone(),
            expected_fee_asset: fee_asset_denomination,
        }
        .build()
        .wrap_err("failed to initialize startup")?;

        let startup_handle = startup::InfoHandle::new(state.subscribe());

        // make submitter object
        let (submitter, submitter_handle) = submitter::Builder {
            shutdown_token: shutdown_handle.token(),
            startup_handle: startup_handle.clone(),
            sequencer_cometbft_endpoint,
            sequencer_grpc_endpoint,
            sequencer_key_path,
            sequencer_address_prefix: sequencer_address_prefix.clone(),
            state: state.clone(),
            metrics,
        }
        .build()
        .wrap_err("failed to initialize submitter")?;

        let ethereum_watcher = watcher::Builder {
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            startup_handle,
            shutdown_token: shutdown_handle.token(),
            state: state.clone(),
            rollup_asset_denom: rollup_asset_denomination,
            bridge_address: sequencer_bridge_address,
            submitter_handle,
        }
        .build()
        .wrap_err("failed to build ethereum watcher")?;

        // make api server
        let state_rx = state.subscribe();
        let api_socket_addr = api_addr.parse::<SocketAddr>().wrap_err_with(|| {
            format!("failed to parse provided `api_addr` string as socket address: `{api_addr}`",)
        })?;
        let api_server = api::start(api_socket_addr, state_rx);

        let service = Self {
            shutdown_token: shutdown_handle.token(),
            api_server,
            submitter,
            ethereum_watcher,
            startup,
            state,
        };

        Ok((service, shutdown_handle))
    }

    // Panic won't happen because `startup_task` is unwraped lazily after checking if it's `Some`.
    #[allow(clippy::missing_panics_doc)]
    pub async fn run(self) {
        let Self {
            shutdown_token,
            api_server,
            submitter,
            ethereum_watcher,
            startup,
            state: _state,
        } = self;

        // Separate the API shutdown signal from the cancellation token because we want it to live
        // until the very end.
        let (api_shutdown_signal, api_shutdown_signal_rx) = oneshot::channel::<()>();
        let mut api_task = tokio::spawn(async move {
            api_server
                .with_graceful_shutdown(async move {
                    let _ = api_shutdown_signal_rx.await;
                })
                .await
                .wrap_err("api server ended unexpectedly")
        });
        info!("spawned API server");

        let mut startup_task = Some(tokio::spawn(startup.run()));
        info!("spawned startup task");

        let mut submitter_task = tokio::spawn(submitter.run());
        info!("spawned submitter task");
        let mut ethereum_watcher_task = tokio::spawn(ethereum_watcher.run());
        info!("spawned ethereum watcher task");

        let shutdown = loop {
            select!(
                o = async { startup_task.as_mut().unwrap().await }, if startup_task.is_none() => {
                    match o {
                        Ok(_) => {
                            info!(task = "startup", "task has exited");
                            startup_task = None;
                        },
                        Err(error) => {
                            error!(task = "startup", %error, "task returned with error");
                            break Shutdown {
                                api_task: Some(api_task),
                                submitter_task: Some(submitter_task),
                                ethereum_watcher_task: Some(ethereum_watcher_task),
                                startup_task: None,
                                api_shutdown_signal,
                                token: shutdown_token,
                            };
                        }
                    }
                }
                o = &mut api_task => {
                    report_exit("api server", o);
                    break Shutdown {
                        api_task: None,
                        submitter_task: Some(submitter_task),
                        ethereum_watcher_task: Some(ethereum_watcher_task),
                        startup_task,
                        api_shutdown_signal,
                       token: shutdown_token
                    }
                }
                o = &mut submitter_task => {
                    report_exit("submitter", o);
                    break Shutdown {
                        api_task: Some(api_task),
                        submitter_task: None,
                        ethereum_watcher_task:Some(ethereum_watcher_task),
                        startup_task,
                        api_shutdown_signal,
                        token: shutdown_token
                    }
                }
                o = &mut ethereum_watcher_task => {
                    report_exit("ethereum watcher", o);
                    break Shutdown {
                        api_task: Some(api_task),
                        submitter_task: Some(submitter_task),
                        ethereum_watcher_task: None,
                        startup_task,
                        api_shutdown_signal,
                        token: shutdown_token
                    }
                }
            );
        };
        shutdown.run().await;
    }
}

/// A handle for instructing the [`Service`] to shut down.
///
/// It is returned along with its related `Service` from [`Service::new`].  The
/// `Service` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
/// when the `ShutdownHandle` is dropped.
pub struct ShutdownHandle {
    token: CancellationToken,
}

impl ShutdownHandle {
    #[must_use]
    fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Returns a clone of the wrapped cancellation token.
    #[must_use]
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Consumes `self` and cancels the wrapped cancellation token.
    pub fn shutdown(self) {
        self.token.cancel();
    }
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        if !self.token.is_cancelled() {
            info!("shutdown handle dropped, issuing shutdown to all services");
        }
        self.token.cancel();
    }
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => info!(task = task_name, "task has exited"),
        Ok(Err(error)) => {
            error!(task = task_name, %error, "task returned with error");
        }
        Err(e) => {
            error!(
                task = task_name,
                error = &e as &dyn std::error::Error,
                "task failed to complete"
            );
        }
    }
}

struct Shutdown {
    api_task: Option<JoinHandle<eyre::Result<()>>>,
    submitter_task: Option<JoinHandle<eyre::Result<()>>>,
    ethereum_watcher_task: Option<JoinHandle<eyre::Result<()>>>,
    startup_task: Option<JoinHandle<eyre::Result<()>>>,
    api_shutdown_signal: oneshot::Sender<()>,
    token: CancellationToken,
}

impl Shutdown {
    const API_SHUTDOWN_TIMEOUT_SECONDS: u64 = 4;
    const ETHEREUM_WATCHER_SHUTDOWN_TIMEOUT_SECONDS: u64 = 5;
    const STARTUP_SHUTDOWN_TIMEOUT_SECONDS: u64 = 1;
    const SUBMITTER_SHUTDOWN_TIMEOUT_SECONDS: u64 = 19;

    async fn run(self) {
        let Self {
            api_task,
            submitter_task,
            ethereum_watcher_task,
            startup_task,
            api_shutdown_signal,
            token,
        } = self;

        token.cancel();

        // Giving startup 1 second to shutdown because it should be very quick.
        if let Some(mut startup_task) = startup_task {
            info!("waiting for startup task to shut down");
            let limit = Duration::from_secs(Self::STARTUP_SHUTDOWN_TIMEOUT_SECONDS);
            match timeout(limit, &mut startup_task).await.map(flatten_result) {
                Ok(Ok(())) => info!("startup exited gracefully"),
                Ok(Err(error)) => error!(%error, "startup exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "startup did not shut down within timeout; killing it"
                    );
                    startup_task.abort();
                }
            }
        }

        // Giving submitter 20 seconds to shutdown because Kubernetes issues a SIGKILL after 30.
        if let Some(mut submitter_task) = submitter_task {
            info!("waiting for submitter task to shut down");
            let limit = Duration::from_secs(Self::SUBMITTER_SHUTDOWN_TIMEOUT_SECONDS);
            match timeout(limit, &mut submitter_task)
                .await
                .map(flatten_result)
            {
                Ok(Ok(())) => info!("withdrawer exited gracefully"),
                Ok(Err(error)) => error!(%error, "withdrawer exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "watcher did not shut down within timeout; killing it"
                    );
                    submitter_task.abort();
                }
            }
        }

        // Giving ethereum watcher 5 seconds to shutdown because Kubernetes issues a SIGKILL after
        // 30.
        if let Some(mut ethereum_watcher_task) = ethereum_watcher_task {
            info!("waiting for watcher task to shut down");
            let limit = Duration::from_secs(Self::ETHEREUM_WATCHER_SHUTDOWN_TIMEOUT_SECONDS);
            match timeout(limit, &mut ethereum_watcher_task)
                .await
                .map(flatten_result)
            {
                Ok(Ok(())) => info!("watcher exited gracefully"),
                Ok(Err(error)) => error!(%error, "watcher exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "watcher did not shut down within timeout; killing it"
                    );
                    ethereum_watcher_task.abort();
                }
            }
        }

        // Giving the API task 4 seconds. 5s for watcher + 20 for submitter + 4s = 29s (out of 30s
        // for k8s).
        if let Some(mut api_task) = api_task {
            info!("sending shutdown signal to API server");
            let _ = api_shutdown_signal.send(());
            let limit = Duration::from_secs(Self::API_SHUTDOWN_TIMEOUT_SECONDS);
            match timeout(limit, &mut api_task).await.map(flatten_result) {
                Ok(Ok(())) => info!("API server exited gracefully"),
                Ok(Err(error)) => error!(%error, "API server exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "API server did not shut down within timeout; killing it"
                    );
                    api_task.abort();
                }
            }
        }
    }
}

pub(crate) fn flatten_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
