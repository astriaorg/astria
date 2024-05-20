use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
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

use crate::{
    api,
    bridge::{
        self,
        Bridge,
    },
    config::Config,
    ethereum::Watcher,
};

pub struct BridgeService {
    // Token to signal all subtasks to shut down gracefully.
    shutdown_token: CancellationToken,
    api_server: api::ApiServer,
    bridge: Bridge,
    ethereum_watcher: Watcher,
}

impl BridgeService {
    pub async fn new(cfg: Config) -> eyre::Result<(Self, ShutdownHandle)> {
        let shutdown_handle = ShutdownHandle::new();
        let Config {
            api_addr, ..
        } = cfg;

        let bridge = bridge::Builder {
            shutdown_token: shutdown_handle.token(),
        }
        .build();

        // make api server
        let state_rx = bridge.subscribe_to_state();
        let api_socket_addr = api_addr.parse::<SocketAddr>().wrap_err_with(|| {
            format!("failed to parse provided `api_addr` string as socket address: `{api_addr}`",)
        })?;
        let api_server = api::start(api_socket_addr, state_rx);

        // TODO: use event_rx in the sequencer submitter
        let (event_tx, _event_rx) = mpsc::channel(100);
        let ethereum_watcher = Watcher::new(
            &cfg.ethereum_contract_address,
            &cfg.ethereum_rpc_endpoint,
            event_tx.clone(),
        )
        .await
        .wrap_err("failed to initialize ethereum watcher")?;

        let bridge = Self {
            shutdown_token: shutdown_handle.token(),
            api_server,
            bridge,
            ethereum_watcher,
        };

        Ok((bridge, shutdown_handle))
    }

    pub async fn run(self) {
        let Self {
            shutdown_token,
            api_server,
            bridge,
            ethereum_watcher,
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

        // TODO remove this
        let mut bridge_task = tokio::spawn(bridge.run());
        info!("spawned bridge withdrawer task");

        let mut ethereum_watcher_task = tokio::spawn(ethereum_watcher.run());
        info!("spawned ethereum watcher task");

        let shutdown = select!(
            o = &mut api_task => {
                report_exit("api server", o);
                Shutdown { api_task: None, ethereum_watcher_task: Some(ethereum_watcher_task), api_shutdown_signal, token: shutdown_token }
            }
            o = &mut bridge_task => {
                report_exit("bridge worker", o);
                Shutdown { api_task: Some(api_task), ethereum_watcher_task: None, api_shutdown_signal, token: shutdown_token }
            }
            o = &mut ethereum_watcher_task => {
                report_exit("ethereum watcher", o);
                Shutdown { api_task: Some(api_task), ethereum_watcher_task: Some(ethereum_watcher_task), api_shutdown_signal, token: shutdown_token }
            }

        );
        shutdown.run().await;
    }
}

/// A handle for instructing the [`Bridge`] to shut down.
///
/// It is returned along with its related `Bridge` from [`Bridge::new`].  The
/// `Bridge` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
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
    ethereum_watcher_task: Option<JoinHandle<eyre::Result<()>>>,
    api_shutdown_signal: oneshot::Sender<()>,
    token: CancellationToken,
}

impl Shutdown {
    const API_SHUTDOWN_TIMEOUT_SECONDS: u64 = 4;
    const BRIDGE_SHUTDOWN_TIMEOUT_SECONDS: u64 = 25;

    async fn run(self) {
        let Self {
            api_task,
            ethereum_watcher_task,
            api_shutdown_signal,
            token,
        } = self;

        token.cancel();

        // Giving bridge 25 seconds to shutdown because Kubernetes issues a SIGKILL after 30.
        if let Some(mut ethereum_watcher_task) = ethereum_watcher_task {
            info!("waiting for watcher task to shut down");
            let limit = Duration::from_secs(Self::BRIDGE_SHUTDOWN_TIMEOUT_SECONDS);
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
        } else {
            info!("watcher task was already dead");
        }

        // Giving the API task 4 seconds. 25 for watcher + 4s = 29s (out of 30s for k8s).
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
        } else {
            info!("API server was already dead");
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
