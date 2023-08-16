use std::net::SocketAddr;

use color_eyre::eyre::{self};
use tokio::task::JoinError;
use tracing::{debug, error, info};

use crate::{
    api::{self, ApiServer},
    searcher::Searcher,
    Config,
};

pub struct Composer {
    api_server: ApiServer,
}

impl Composer {
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    pub async fn run_until_stopped(cfg: &Config) -> Result<(), eyre::Error> {
        debug!("creating searcher");
        let mut searcher = Searcher::setup_searcher(&cfg)?;
        let rollup_clients = Searcher::setup_rollup_clients(&cfg).await?;
        searcher.run(rollup_clients).await?;
        let searcher_status = searcher.subscribe();

        debug!("creating API server");
        let api_server = api::start(cfg.api_listen_addr, searcher_status);
        debug!(
                listen_addr = %api_server.local_addr(),
                "API server listening");
        api_server.await?;

        Ok(())
    }
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => info!(task = task_name, "task exited successfully"),
        Ok(Err(e)) => {
            error!(
                error.cause_chain = ?e,
                error.message = %e,
                task = task_name,
                "task failed",
            );
        }
        Err(e) => {
            error!(
                error.cause_chain = ?e,
                error.message = %e,
                task = task_name,
                "task failed to complete",
            );
        }
    }
}
