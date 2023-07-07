use std::{
    net::SocketAddr,
    sync::Arc,
};

use ethers::types::Transaction;
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinError,
};
use tracing::{
    error,
    info,
};

use self::collector::TxCollector;
use crate::config::searcher::{
    self as config,
    Config,
};

mod api;
mod bundler;
mod collector;
mod executor;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid config")]
    InvalidConfig(#[source] config::Error),
    #[error("task error")]
    TaskError(#[source] JoinError),
    #[error("api error")]
    ApiError(#[source] hyper::Error),
    #[error("collector error")]
    CollectorError(#[source] collector::Error),
}

#[derive(Debug)]
pub(crate) struct State();

pub struct Searcher {
    state: Arc<State>,
    api_url: SocketAddr,
    tx_collector: TxCollector,
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// Returns a `searcher::Error::InvalidConfig` if there is an error constructing `api_url` from
    /// the port specified in config.
    pub async fn new(cfg: &Config) -> Result<Self, Error> {
        // configure rollup tx collector
        let tx_collector = TxCollector::new(&cfg.execution_ws_url)
            .await
            .map_err(Error::CollectorError)?;
        // configure rollup tx bundler
        // configure rollup tx executor

        // init searcher state
        let state = Arc::new(State());

        // parse api url from config
        let api_url = Config::api_url(cfg.api_port).map_err(Error::InvalidConfig)?;

        Ok(Self {
            state,
            api_url,
            tx_collector,
        })
    }

    /// Runs the Searcher and blocks until all subtasks have exited.
    /// - api server
    /// TODO:
    /// - rollup tx collector
    /// - rollup tx bundler
    /// - rollup tx executor
    ///
    /// # Errors
    ///
    /// Returns a `searcher::Error` if the Searcher fails to start or if any of the subtasks fail
    /// and cannot be recovered.
    pub async fn run(self) {
        let Self {
            state,
            api_url,
            tx_collector,
        } = self;

        // collector -> bundler
        let (event_tx, _event_rx): (mpsc::Sender<Event>, mpsc::Receiver<Event>) =
            mpsc::channel(512);
        // bundler -> executor
        let (_action_tx, _action_rx): (oneshot::Sender<Action>, oneshot::Receiver<Action>) =
            oneshot::channel();

        let api_task = tokio::spawn(api::run(api_url, state.clone()));
        let collector_task = tokio::spawn(tx_collector.run(event_tx));

        tokio::select! {
            o = api_task => {
                match o {
                    Ok(task_result) => report_exit("api server", task_result.map_err(Error::ApiError)),
                    Err(e) => report_exit("api server", Err(Error::TaskError(e))),

                }
            }
            o = collector_task => {
                match o {
                    Ok(task_result) => report_exit("rollup tx collector", task_result.map_err(Error::CollectorError)),
                    Err(e) => report_exit("rollup tx collector", Err(Error::TaskError(e))),
                }
            }
        }
    }
}

fn report_exit(task_name: &str, outcome: Result<(), Error>) {
    match outcome {
        Ok(()) => info!(task = task_name, "task exited successfully"),
        Err(e) => match e {
            Error::TaskError(join_err) => {
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
    SendSequencerSecondaryTx,
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
