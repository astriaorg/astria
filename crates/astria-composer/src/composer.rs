use std::net::SocketAddr;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::task::JoinError;
use tracing::{
    debug,
    error,
    info,
};

use crate::{
    api::{
        self,
        ApiServer,
    },
    searcher::Searcher,
    Config,
};

/// Composer is a service responsible for submitting transactions to the Astria
/// Shared Sequencer.
pub struct Composer {
    /// ApiServer is used for monitoring status of the Composer service.
    api_server: ApiServer,
    /// Searcher establishes connections to individual rollup nodes, receiving
    /// pending transactions from them and wraps them as sequencer transactions
    /// for submission.
    searcher: Searcher,
}

impl Composer {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// An error is returned if the searcher fails to be initialized.
    /// See `[Searcher::from_config]` for its error scenarios.
    pub fn from_config(cfg: Config) -> eyre::Result<Self> {
        // parse api url from config
        debug!("creating searcher");
        let searcher = Searcher::from_config(&cfg).wrap_err("failed to initialize searcher")?;

        let searcher_status = searcher.subscribe_to_state();

        debug!("creating API server");
        let api_server = api::start(cfg.api_listen_addr, searcher_status);
        debug!(
            listen_addr = %api_server.local_addr(),
            "API server listening"
        );

        Ok(Self {
            api_server,
            searcher,
        })
    }

    /// Returns the socket address the api server is served over
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Runs the composer.
    ///
    /// Currently only exits if the api server or searcher stop unexpectedly.
    pub async fn run_until_stopped(self) {
        let Self {
            api_server,
            searcher,
        } = self;

        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });
        let searcher_task = tokio::spawn(searcher.run());

        tokio::select! {
            o = api_task => report_exit("api server", o),
            o = searcher_task => report_exit("searcher", o),
        }
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
