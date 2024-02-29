use std::net::SocketAddr;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::task::JoinError;
use tracing::info;

use crate::{
    api,
    config::Config,
    relayer::Relayer,
};

pub struct SequencerRelayer {
    api_server: api::ApiServer,
    relayer: Relayer,
}

impl SequencerRelayer {
    /// Instantiates a new `SequencerRelayer`.
    ///
    /// # Errors
    ///
    /// Returns an error if constructing the inner relayer type failed.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        let relayer = Relayer::new(cfg)
            .await
            .wrap_err("failed to create relayer")?;
        let state_rx = relayer.subscribe_to_state();
        let api_server = api::start(cfg.rpc_port, state_rx);
        Ok(Self {
            api_server,
            relayer,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    pub async fn run(self) {
        let Self {
            api_server,
            relayer,
        } = self;
        // Wrap the API server in an async block so we can easily turn the result
        // of the future into an eyre report.
        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });
        info!("spawned API server");

        let relayer_task = tokio::spawn(relayer.run());
        info!("spawned relayer task");

        tokio::select!(
            o = api_task => report_exit("api server", o),
            o = relayer_task => report_exit("relayer worker", o),
        );
    }
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!(task = task_name, "task has exited"),
        Ok(Err(error)) => {
            tracing::error!(task = task_name, %error, "task returned with error");
        }
        Err(e) => {
            tracing::error!(
                task = task_name,
                error = &e as &dyn std::error::Error,
                "task failed to complete"
            );
        }
    }
}
