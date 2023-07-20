use std::net::SocketAddr;

use astria_sequencer::transaction::Unsigned as SequencerUnsignedTx;
use astria_sequencer_client::Client as SequencerClient;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use ethers::types::Transaction;
use tokio::{
    sync::broadcast::{
        self,
        Receiver,
        Sender,
    },
    task::JoinError,
};
use tracing::{
    error,
    info,
};

use self::{
    bundler::Bundler,
    collector::TxCollector,
    executor::SequencerExecutor,
};
use crate::Config;
mod api;
mod bundler;
mod collector;
mod executor;

// #[derive(Debug)]
pub struct Searcher {
    api_server: api::ApiServer,
    tx_collector: TxCollector,
    bundler: Bundler,
    executor: SequencerExecutor,
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// - `Error::CollectorError` if there is an error initializing the tx collector.
    /// - `Error::BundlerError` if there is an error initializing the tx bundler.
    /// - `Error::SequencerClientInit` if there is an error initializing the sequencer client.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        // configure rollup tx collector
        let tx_collector = TxCollector::new(&cfg.execution_ws_url)
            .await
            .wrap_err("failed to start tx collector")?;

        // configure rollup tx bundler
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let bundler = Bundler::new(
            sequencer_client.clone(),
            cfg.sequencer_address.to_string(),
            cfg.chain_id.clone(),
        )
        .wrap_err("failed constructing bundler")?;

        let executor = SequencerExecutor::new(sequencer_client.clone(), &cfg.sequencer_secret);

        // parse api url from config
        let api_server = api::start(cfg.api_port);

        Ok(Self {
            api_server,
            tx_collector,
            bundler,
            executor,
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
    pub async fn run(self) {
        let Self {
            api_server,
            tx_collector,
            bundler,
            executor,
        } = self;

        // collector -> bundler
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = broadcast::channel(512);
        // bundler -> executor
        let (action_tx, action_rx): (Sender<Action>, Receiver<Action>) = broadcast::channel(512);

        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended upexpectedly") });
        let collector_task = tokio::spawn(tx_collector.run(event_tx));
        let bundler_task = tokio::spawn(bundler.run(event_rx, action_tx));
        let executor_task = tokio::spawn(executor.run(action_rx));

        tokio::select! {
            o = api_task => report_exit("api server", o),
            o = collector_task => report_exit("rollup tx collector", o),
            o = bundler_task => report_exit("bundler", o),
            o = executor_task => report_exit("sequencer executor", o),
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

#[derive(Debug, Clone)]
pub enum Event {
    NewTx(Transaction),
}

#[derive(Debug, Clone)]
pub enum Action {
    SendSequencerSecondaryTx(SequencerUnsignedTx),
}

#[cfg(test)]
mod tests {
    use crate::{
        config::searcher::Config,
        searcher::Searcher,
    };

    #[tokio::test]
    async fn new_from_valid_config() {
        let cfg = Config::default();
        let searcher = Searcher::new(&cfg).await;
        assert!(searcher.is_ok());
        // assert!(dbg!(searcher).is_ok());
    }
}
