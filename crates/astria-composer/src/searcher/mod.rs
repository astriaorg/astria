use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_sequencer::{
    accounts::types::Address,
    transaction::Unsigned as SequencerUnsignedTx,
};
use astria_sequencer_client::Client as SequencerClient;
use ed25519_consensus::SigningKey;
use ethers::{
    prelude::rand::seq,
    types::Transaction,
};
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
    executor::SequencerExecutor, error::ComposerError,
};
use crate::config::searcher::{
    self as config,
    Config,
};
mod api;
mod bundler;
mod collector;
mod executor;
mod error;

pub struct Searcher {
    api_url: SocketAddr,
    tx_collector: TxCollector,
    bundler: Bundler,
    executor: SequencerExecutor,
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// - `Error::InvalidConfig` if there is an error constructing `api_url` from
    /// the port specified in config.
    /// - `Error::CollectorError` if there is an error initializing the tx collector.
    /// - `Error::BundlerError` if there is an error initializing the tx bundler.
    /// - `Error::SequencerClientInit` if there is an error initializing the sequencer client.
    pub async fn new(cfg: &Config) -> Result<Self, ComposerError> {
        // configure rollup tx collector
        let tx_collector = TxCollector::new(&cfg.execution_ws_url)
            .await
            .map_err(ComposerError::CollectorError)?;

        // configure rollup tx bundler
        let sequencer_client = Arc::new(
            SequencerClient::new(&cfg.sequencer_url).map_err(|_| ComposerError::SequencerClientInit)?,
        );

        let bundler = Bundler::new(
            sequencer_client.clone(),
            cfg.sequencer_address.to_string(),
            cfg.chain_id.clone(),
        )
        .map_err(ComposerError::BundlerError)?;

        // configure rollup tx executor
        let executor = SequencerExecutor::new(sequencer_client.clone(), &cfg.sequencer_secret);

        // parse api url from config
        let api_url = Config::api_url(cfg.api_port).map_err(ComposerError::InvalidConfig)?;

        Ok(Self {
            api_url,
            tx_collector,
            bundler,
            executor,
        })
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
            api_url,
            tx_collector,
            bundler,
            executor,
        } = self;

        // collector -> bundler
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = broadcast::channel(512);
        // bundler -> executor
        let (action_tx, action_rx): (Sender<Action>, Receiver<Action>) = broadcast::channel(512);

        let api_event_rx = event_tx.subscribe();
        let api_action_rx = action_tx.subscribe();

        let api_task = tokio::spawn(api::run(api_url, api_event_rx, api_action_rx));
        let collector_task = tokio::spawn(tx_collector.run(event_tx));
        let bundler_task = tokio::spawn(bundler.run(event_rx, action_tx));
        let executor_task = tokio::spawn(executor.run(action_rx));

        tokio::select! {
            o = api_task => {
                match o {
                    Ok(task_result) => report_exit("api server", task_result.map_err(ComposerError::ApiError)),
                    Err(e) => report_exit("api server", Err(ComposerError::TaskError(e))),

                }
            }
            o = collector_task => {
                match o {
                    Ok(task_result) => report_exit("rollup tx collector", task_result.map_err(ComposerError::CollectorError)),
                    Err(e) => report_exit("rollup tx collector", Err(ComposerError::TaskError(e))),
                }
            }
            o = bundler_task => {
                match o {
                    Ok(task_result) => report_exit("bundler", task_result.map_err(ComposerError::BundlerError)),
                    Err(e) => report_exit("bundler", Err(ComposerError::TaskError(e))),
                }
            }
            o = executor_task => {
                match o {
                    Ok(task_result) => {
                        match task_result {
                            Err(executor::Error::InvalidNonce(_nonce)) => {
                                todo!("handle invalid nonce by resetting bundler's nonce (reset_nonce) and readding the tx to event queue");
                            },
                            result => report_exit("executor", result.map_err(ComposerError::ExecutorError)),
                        }
                    },
                    Err(e) => report_exit("sequencer executor", Err(ComposerError::TaskError(e))),
                }
            }
        }
    }
}

fn report_exit(task_name: &str, outcome: Result<(), ComposerError>) {
    match outcome {
        Ok(()) => info!(task = task_name, "task exited successfully"),
        Err(e) => match e {
            ComposerError::TaskError(join_err) => {
                error!(task = task_name, error.msg = %join_err, error.cause = ?join_err, "task failed to complete");
            }
            service_err => {
                error!(task = task_name, error.msg = %service_err, error.cause = ?service_err, "task exited with error");
            }
        },
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
    }
}
