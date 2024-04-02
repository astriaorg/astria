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
    relayer::{
        self,
        Relayer,
    },
};

pub struct SequencerRelayer {
    api_server: api::ApiServer,
    relayer: Relayer,
    shutdown_token: CancellationToken,
}

impl SequencerRelayer {
    /// Instantiates a new `SequencerRelayer`.
    ///
    /// # Errors
    ///
    /// Returns an error if constructing the inner relayer type failed.
    pub fn new(cfg: Config) -> eyre::Result<(Self, ShutdownHandle)> {
        let shutdown_handle = ShutdownHandle::new();
        let Config {
            cometbft_endpoint,
            sequencer_grpc_endpoint,
            celestia_endpoint,
            celestia_bearer_token,
            block_time,
            relay_only_validator_key_blocks,
            validator_key_file,
            api_addr,
            pre_submit_path,
            post_submit_path,
            ..
        } = cfg;

        let validator_key_path = relay_only_validator_key_blocks.then_some(validator_key_file);
        let relayer = relayer::Builder {
            shutdown_token: shutdown_handle.token(),
            celestia_endpoint,
            celestia_bearer_token,
            cometbft_endpoint,
            sequencer_poll_period: Duration::from_millis(block_time),
            sequencer_grpc_endpoint,
            validator_key_path,
            pre_submit_path,
            post_submit_path,
        }
        .build()
        .wrap_err("failed to create relayer")?;

        let state_rx = relayer.subscribe_to_state();
        let api_socket_addr = api_addr.parse::<SocketAddr>().wrap_err_with(|| {
            format!("failed to parse provided `api_addr` string as socket address: `{api_addr}`",)
        })?;
        let api_server = api::start(api_socket_addr, state_rx);
        let relayer = Self {
            api_server,
            relayer,
            shutdown_token: shutdown_handle.token(),
        };
        Ok((relayer, shutdown_handle))
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Runs Sequencer Relayer.
    pub async fn run(self) {
        let Self {
            api_server,
            relayer,
            shutdown_token,
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

        let mut relayer_task = tokio::spawn(relayer.run());
        info!("spawned relayer task");

        let shutdown = select!(
            o = &mut api_task => {
                report_exit("api server", o);
                ShutDown { api_task: None, relayer_task: Some(relayer_task), api_shutdown_signal, shutdown_token }
            }
            o = &mut relayer_task => {
                report_exit("relayer worker", o);
                ShutDown { api_task: Some(api_task), relayer_task: None, api_shutdown_signal, shutdown_token }
            }

        );
        shutdown.run().await;
    }
}

/// A handle for instructing the [`SequencerRelayer`] to shut down.
///
/// It is returned along with its related `SequencerRelayer` from [`SequencerRelayer::new`].  The
/// `SequencerRelayer` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
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

struct ShutDown {
    api_task: Option<JoinHandle<eyre::Result<()>>>,
    relayer_task: Option<JoinHandle<eyre::Result<()>>>,
    api_shutdown_signal: oneshot::Sender<()>,
    shutdown_token: CancellationToken,
}

impl ShutDown {
    async fn run(self) {
        let Self {
            api_task,
            relayer_task,
            api_shutdown_signal,
            shutdown_token,
        } = self;
        shutdown_token.cancel();
        // Giving relayer 25 seconds to shutdown because Kubernetes issues a SIGKILL after 30.
        if let Some(mut relayer_task) = relayer_task {
            info!("waiting for relayer task to shut down");
            let limit = Duration::from_secs(25);
            match timeout(limit, &mut relayer_task)
                .await
                .map(crate::utils::flatten)
            {
                Ok(Ok(())) => info!("relayer exited gracefully"),
                Ok(Err(error)) => error!(%error, "relayer exited with an error"),
                Err(_) => {
                    error!(
                        timeout_secs = limit.as_secs(),
                        "relayer did not shut down within timeout; killing it"
                    );
                    relayer_task.abort();
                }
            }
        } else {
            info!("relayer task was already dead");
        }

        // Giving the API task another 4 seconds. 25 for relayer + 4s = 29s (out of 30s for k8s).
        if let Some(mut api_task) = api_task {
            info!("sending shutdown signal to API server");
            let _ = api_shutdown_signal.send(());
            let limit = Duration::from_secs(4);
            match timeout(limit, &mut api_task)
                .await
                .map(crate::utils::flatten)
            {
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
