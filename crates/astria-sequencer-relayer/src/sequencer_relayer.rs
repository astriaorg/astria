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
    task::{
        JoinError,
        JoinHandle,
    },
    time::timeout,
};
use tokio_util::sync::{
    CancellationToken,
    WaitForCancellationFuture,
};
use tracing::{
    error,
    info,
    instrument,
};

use crate::{
    api,
    config::Config,
    metrics::Metrics,
    relayer::{
        self,
        Relayer,
    },
};

pub struct SequencerRelayer {
    api: api::Serve,
    api_shutdown_token: CancellationToken,
    relayer: Relayer,
    shutdown_handle: ShutdownHandle,
}

impl SequencerRelayer {
    /// Instantiates a new `SequencerRelayer`.
    ///
    /// # Errors
    ///
    /// Returns an error if constructing the inner relayer type failed.
    pub async fn new(
        cfg: Config,
        metrics: &'static Metrics,
    ) -> eyre::Result<(Self, ShutdownHandle)> {
        let shutdown_handle = ShutdownHandle::new();
        let rollup_filter = cfg.only_include_rollups()?;
        let Config {
            sequencer_chain_id,
            celestia_chain_id,
            cometbft_endpoint,
            sequencer_grpc_endpoint,
            celestia_app_grpc_endpoint,
            celestia_app_key_file,
            block_time,
            api_addr,
            submission_state_path,
            celestia_default_min_gas_price,
            ..
        } = cfg;

        let relayer = relayer::Builder {
            relayer_shutdown_token: shutdown_handle.token.child_token(),
            sequencer_chain_id,
            celestia_chain_id,
            celestia_default_min_gas_price,
            celestia_app_grpc_endpoint,
            celestia_app_key_file,
            cometbft_endpoint,
            sequencer_poll_period: Duration::from_millis(block_time),
            sequencer_grpc_endpoint,
            rollup_filter,
            submission_state_path,
            metrics,
        }
        .build()
        .wrap_err("failed to create relayer")?;

        let api_shutdown_token = CancellationToken::new();
        let api = api::serve(
            &api_addr,
            relayer.subscribe_to_state(),
            api_shutdown_token.child_token(),
        )
        .await
        .wrap_err("failed to start API server")?;
        let relayer = Self {
            api,
            api_shutdown_token,
            relayer,
            shutdown_handle: shutdown_handle.clone(),
        };
        Ok((relayer, shutdown_handle))
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.api.local_addr()
    }

    /// Runs Sequencer Relayer.
    pub async fn run(self) {
        let Self {
            api,
            api_shutdown_token,
            relayer,
            shutdown_handle,
        } = self;
        let (mut api_task, mut relayer_task) = spawn_tasks(api, relayer);

        let shutdown = select!(
            o = &mut api_task => {
                report_exit("api server", o);
                Shutdown { api_task: None, relayer_task: Some(relayer_task), api_shutdown_token, handle: shutdown_handle }
            }
            o = &mut relayer_task => {
                report_exit("relayer worker", o);
                Shutdown { api_task: Some(api_task), relayer_task: None, api_shutdown_token, handle: shutdown_handle }
            }

        );
        shutdown.run().await;
    }
}

#[instrument(skip_all)]
fn spawn_tasks(
    api: api::Serve,
    relayer: Relayer,
) -> (JoinHandle<eyre::Result<()>>, JoinHandle<eyre::Result<()>>) {
    let api_task = tokio::spawn(async move { api.await.wrap_err("API server exited with error") });
    let relayer_task = tokio::spawn(relayer.run());
    info!("spawned relayer task");

    (api_task, relayer_task)
}

/// A handle for instructing the [`SequencerRelayer`] to shut down.
///
/// It is returned along with its related `SequencerRelayer` from [`SequencerRelayer::new`].  The
/// `SequencerRelayer` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
/// when the `ShutdownHandle` is dropped.
#[derive(Clone)]
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

    /// Returns a `Future` that gets fulfilled when cancellation is requested.
    ///
    /// See [`CancellationToken::cancelled`] for further details.
    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.token.cancelled()
    }

    /// Consumes `self` and cancels the wrapped cancellation token.
    ///
    /// See [`CancellationToken::cancel`] for further details.
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
    api_shutdown_token: CancellationToken,
    relayer_task: Option<JoinHandle<eyre::Result<()>>>,
    handle: ShutdownHandle,
}

impl Shutdown {
    #[instrument(skip_all)]
    async fn run(self) {
        let Self {
            api_task,
            api_shutdown_token,
            relayer_task,
            handle,
        } = self;
        handle.shutdown();
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
            api_shutdown_token.cancel();
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
