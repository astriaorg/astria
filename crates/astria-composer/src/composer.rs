use std::net::SocketAddr;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use ethers::providers::Ws;
use tokio::task::JoinError;
use tracing::{
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

pub struct Composer {
    api_server: ApiServer,
    searcher: Searcher<Ws>,
}

impl Composer {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// - `Error::CollectorError` if there is an error initializing the tx collector.
    /// - `Error::BundlerError` if there is an error initializing the tx bundler.
    /// - `Error::SequencerClientInit` if there is an error initializing the sequencer client.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        // parse api url from config
        let api_server = api::start(cfg.api_port);
        let searcher = Searcher::<Ws>::new_ws(&cfg)
            .await
            .wrap_err("failed to initialize searcher")?;

        Ok(Self {
            api_server,
            searcher,
        })
    }

    pub fn api_listen_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Runs the Searcher and blocks until all subtasks have exited:
    /// - api server
    /// - tx collector
    /// - bundler
    /// - executor
    ///
    /// # Errors
    ///
    /// - `searcher::Error` if the Searcher fails to start or if any of the subtasks fail
    /// and cannot be recovered.
    pub async fn run_until_stopped(self) {
        let Self {
            api_server,
            searcher,
        } = self;

        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended upexpectedly") });
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
            )
        }
        Err(e) => {
            error!(
                error.cause_chain = ?e,
                error.message = %e,
                task = task_name,
                "task failed to complete",
            )
        }
    }
}
