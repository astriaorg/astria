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
    signal::unix::{
        signal,
        SignalKind,
    },
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
    pub fn new(cfg: Config) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
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
            shutdown_token: shutdown_token.clone(),
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
        Ok(Self {
            api_server,
            relayer,
            shutdown_token,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Run Sequencer Relayer.
    ///
    /// # Panics
    /// Panics if a signal listener could not be constructed (usually if this binary is not run on a
    /// Unix).
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

        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on Unix?",
        );

        let shutdown = select!(
            _ = sigterm.recv() => {
                info!("received SIGTERM, issuing shutdown to all services");
                Shutdown { api_task: Some(api_task), relayer_task: Some(relayer_task), api_shutdown_signal, shutdown_token }
            },

            o = &mut api_task => {
                report_exit("api server", o);
                Shutdown { api_task: None, relayer_task: Some(relayer_task), api_shutdown_signal, shutdown_token }
            }
            o = &mut relayer_task => {
                report_exit("relayer worker", o);
                Shutdown { api_task: Some(api_task), relayer_task: None, api_shutdown_signal, shutdown_token }
            }

        );
        shutdown.run().await;
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
    relayer_task: Option<JoinHandle<eyre::Result<()>>>,
    api_shutdown_signal: oneshot::Sender<()>,
    shutdown_token: CancellationToken,
}

impl Shutdown {
    async fn run(self) {
        let Self {
            api_task,
            relayer_task,
            api_shutdown_signal,
            shutdown_token
        } = self;
        shutdown_token.cancel();
        // Giving relayer 25 seconds to shutdown because Kubernetes issues a SIGKILL after 30.
        if let Some(mut relayer_task) = relayer_task {
            info!("waiting for relayer task to shut down");
            match timeout(Duration::from_secs(25), &mut relayer_task)
                .await
                .map(crate::utils::flatten)
            {
                Ok(Ok(())) => info!("relayer exited gracefully"),
                Ok(Err(error)) => error!(%error, "relayer exited with an error"),
                Err(_) => {
                    error!("relayer did not shut down after 25 seconds; killing it");
                    relayer_task.abort();
                }
            }
            info!("sending shutdown signal to API server");
        } else {
            info!("relayer task was already dead");
        }

        // Giving the API task another 4 seconds. 25 for relayer + 4s = 29s (out of 30s for k8s).
        if let Some(mut api_task) = api_task {
            info!("sending shutdown signal to API server");
            let _ = api_shutdown_signal.send(());
            match timeout(Duration::from_secs(4), &mut api_task)
                .await
                .map(crate::utils::flatten)
            {
                Ok(Ok(())) => info!("api server exited gracefully"),
                Ok(Err(error)) => error!(%error, "api server exited with an error"),
                Err(_) => {
                    error!("api server did not shut down after 25 seconds; killing it");
                    api_task.abort();
                }
            }
        } else {
            info!("API server was already dead");
        }
    }
}
