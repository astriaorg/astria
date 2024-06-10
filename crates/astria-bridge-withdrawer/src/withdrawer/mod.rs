use std::{
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use astria_core::primitive::v1::{
    asset::{
        self,
        Denom,
    },
    Address,
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
};

mod batch;
mod ethereum;
mod state;
mod submitter;

pub struct Service {
    // Token to signal all subtasks to shut down gracefully.
    shutdown_token: CancellationToken,
    api_server: api::ApiServer,
    submitter: Submitter,
    ethereum_watcher: watcher::Watcher,
    state: Arc<State>,
}

impl Service {
    /// Instantiates a new `Service`.
    ///
    /// # Errors
    ///
    /// - If the provided `api_addr` string cannot be parsed as a socket address.
    pub fn new(cfg: Config) -> eyre::Result<(Self, ShutdownHandle)> {
        let shutdown_handle = ShutdownHandle::new();
        let Config {
            api_addr,
            sequencer_cometbft_endpoint,
            sequencer_chain_id,
            sequencer_key_path,
            fee_asset_denomination,
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            rollup_asset_denomination,
            min_expected_fee_asset_balance,
            ..
        } = cfg;

        let state = Arc::new(State::new());

        // make submitter object
        let (submitter, submitter_handle) = submitter::Builder {
            shutdown_token: shutdown_handle.token(),
            sequencer_cometbft_endpoint,
            sequencer_chain_id,
            sequencer_key_path,
            state: state.clone(),
            expected_fee_asset_id: asset::Id::from_denom(&fee_asset_denomination),
            min_expected_fee_asset_balance: u128::from(min_expected_fee_asset_balance),
        }
        .build()
        .wrap_err("failed to initialize submitter")?;

        let sequencer_bridge_address = Address::try_from_bech32m(&cfg.sequencer_bridge_address)
            .wrap_err("failed to parse sequencer bridge address")?;

        let ethereum_watcher = watcher::Builder {
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            submitter_handle,
            shutdown_token: shutdown_handle.token(),
            state: state.clone(),
            rollup_asset_denom: rollup_asset_denomination
                .parse::<Denom>()
                .wrap_err("failed to parse ROLLUP_ASSET_DENOMINATION as Denom")?,
            bridge_address: sequencer_bridge_address,
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
            state,
        };

        Ok((service, shutdown_handle))
    }

    pub async fn run(self) {
        let Self {
            shutdown_token,
            api_server,
            submitter,
            ethereum_watcher,
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

        let mut submitter_task = tokio::spawn(submitter.run());
        info!("spawned submitter task");
        let mut ethereum_watcher_task = tokio::spawn(ethereum_watcher.run());
        info!("spawned ethereum watcher task");

        let shutdown = select!(
            o = &mut api_task => {
                report_exit("api server", o);
                Shutdown {
                    api_task: None,
                    submitter_task: Some(submitter_task),
                    ethereum_watcher_task: Some(ethereum_watcher_task),
                    api_shutdown_signal,
                   token: shutdown_token
                }
            }
            o = &mut submitter_task => {
                report_exit("submitter", o);
                Shutdown {
                    api_task: Some(api_task),
                    submitter_task: None,
                    ethereum_watcher_task:Some(ethereum_watcher_task),
                    api_shutdown_signal,
                    token: shutdown_token
                }
            }
            o = &mut ethereum_watcher_task => {
                report_exit("ethereum watcher", o);
                Shutdown {
                    api_task: Some(api_task),
                    submitter_task: Some(submitter_task),
                    ethereum_watcher_task: None,
                    api_shutdown_signal,
                    token: shutdown_token
                }
            }

        );
        shutdown.run().await;
    }
}

#[derive(Debug)]
pub struct SequencerStartupInfo {
    pub fee_asset_id: asset::Id,
    pub next_batch_rollup_height: u64,
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
    api_shutdown_signal: oneshot::Sender<()>,
    token: CancellationToken,
}

impl Shutdown {
    const API_SHUTDOWN_TIMEOUT_SECONDS: u64 = 4;
    const ETHEREUM_WATCHER_SHUTDOWN_TIMEOUT_SECONDS: u64 = 5;
    const SUBMITTER_SHUTDOWN_TIMEOUT_SECONDS: u64 = 20;

    async fn run(self) {
        let Self {
            api_task,
            submitter_task,
            ethereum_watcher_task,
            api_shutdown_signal,
            token,
        } = self;

        token.cancel();

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
        } else {
            info!("submitter task was already dead");
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
        } else {
            info!("watcher task was already dead");
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

/// Constructs an [`Address`] prefixed by `"astria"`.
#[cfg(test)]
pub(crate) fn astria_address(array: [u8; astria_core::primitive::v1::ADDRESS_LEN]) -> Address {
    use astria_core::primitive::v1::ASTRIA_ADDRESS_PREFIX;
    Address::builder()
        .array(array)
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}
