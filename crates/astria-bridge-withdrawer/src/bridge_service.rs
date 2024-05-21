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

use crate::{
    api,
    config::Config,
    executor::{
        self,
        Executor,
    },
};

pub struct BridgeService {
    // Token to signal all subtasks to shut down gracefully.
    shutdown_token: CancellationToken,
    api_server: api::ApiServer,
    executor: Executor,
}

impl BridgeService {
    pub fn new(cfg: Config) -> eyre::Result<(Self, ShutdownHandle)> {
        let shutdown_handle = ShutdownHandle::new();
        let Config {
            api_addr, ..
        } = cfg;

        // make bridge object
        // TODO: add more fields
        let (executor, executor_handle) = executor::Builder {
            shutdown_token: shutdown_handle.token(),
            cometbft_endpoint: cfg.cometbft_endpoint,
            sequencer_chain_id: cfg.sequencer_chain_id,
            sequencer_key_path: cfg.sequencer_key_path,
        }
        .build()
        .wrap_err("failed to create bridge")?;

        // make api server
        let state_rx = executor.subscribe_to_state();
        let api_socket_addr = api_addr.parse::<SocketAddr>().wrap_err_with(|| {
            format!("failed to parse provided `api_addr` string as socket address: `{api_addr}`",)
        })?;
        let api_server = api::start(api_socket_addr, state_rx);

        let bridge = Self {
            shutdown_token: shutdown_handle.token(),
            api_server,
            executor,
        };

        Ok((bridge, shutdown_handle))
    }

    pub async fn run(self) {
        let Self {
            shutdown_token,
            api_server,
            executor,
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

        let mut executor_task = tokio::spawn(executor.run());
        info!("spawned bridge withdrawer task");

        let shutdown = select!(
            o = &mut api_task => {
                report_exit("api server", o);
                Shutdown { api_task: None, executor_task: Some(executor_task), api_shutdown_signal, shutdown_token }
            }
            o = &mut executor_task => {
                report_exit("bridge worker", o);
                Shutdown { api_task: Some(api_task), executor_task: None, api_shutdown_signal, shutdown_token }
            }

        );
        shutdown.run().await;
    }
}

/// A handle for instructing the [`BridgeService`] to shut down.
///
/// It is returned along with its related `BridgeService` from [`BridgeService::new`].  The
/// `BridgeService` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
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
    executor_task: Option<JoinHandle<eyre::Result<()>>>,
    api_shutdown_signal: oneshot::Sender<()>,
    shutdown_token: CancellationToken,
}

impl Shutdown {
    const API_SHUTDOWN_TIMEOUT_SECONDS: u64 = 4;
    const EXECUTOR_SHUTDOWN_TIMEOUT_SECONDS: u64 = 25;

    async fn run(self) {
        let Self {
            api_task,
            executor_task,
            api_shutdown_signal,
            shutdown_token,
        } = self;

        shutdown_token.cancel();

        // Giving executor 25 seconds to shutdown because Kubernetes issues a SIGKILL after 30.
        if let Some(mut executor_task) = executor_task {
            info!("waiting for executor task to shut down");
            let limit = Duration::from_secs(Self::EXECUTOR_SHUTDOWN_TIMEOUT_SECONDS);
            match timeout(limit, &mut executor_task).await.map(flatten_result) {
                Ok(Ok(())) => info!("bridge exited gracefully"),
                Ok(Err(error)) => error!(%error, "bridge exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "bridge did not shut down within timeout; killing it"
                    );
                    executor_task.abort();
                }
            }
        } else {
            info!("bridge task was already dead");
        }

        // Giving the API task 4 seconds. 25 for bridge + 4s = 29s (out of 30s for k8s).
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
