use std::{
    net::SocketAddr,
    sync::Arc,
};

use color_eyre::eyre;
use ethers::providers::{
    JsonRpcClient,
    Provider,
    PubsubClient,
    Ws,
};
use tokio::{
    sync::mpsc::{
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

use super::{
    api,
    bundler::{
        self,
        Bundler,
    },
    collector::{
        self,
        TxCollector,
    },
    executor::{
        self,
        SequencerExecutor,
    },
    Action,
    Event,
};
use crate::{
    api::ApiServer,
    Config,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid config")]
    InvalidConfig,
    #[error("task error")]
    TaskError(#[source] JoinError),
    #[error("api error")]
    ApiError(#[source] hyper::Error),
    #[error("sequencer client init failed")]
    SequencerClientInit,
}

pub struct Searcher<P>
where
    P: PubsubClient + JsonRpcClient + Clone,
{
    api_server: ApiServer,
    tx_collector: TxCollector<P>,
    bundler: Bundler,
    executor: SequencerExecutor,
}

impl Searcher<Ws> {
    /// Constructs a new Searcher service from config.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        // configure rollup tx collector
        let provider = Provider::<Ws>::connect(format!("ws://{}", cfg.execution_ws_url)).await?;
        info!(?cfg.execution_ws_url, "connected to execution node");
        let tx_collector = TxCollector::<Ws>::new(provider);

        // configure rollup tx bundler

        let bundler = Bundler::new(cfg.sequencer_address.to_string(), cfg.chain_id.clone())?;

        // configure rollup tx executor
        let executor = SequencerExecutor::new(cfg.sequencer_url.clone(), &cfg.sequencer_secret)?;

        // parse api url from config
        // TODO: use ApiServer type
        let api_url = Config::api_url(cfg.api_port).map_err(Error::InvalidConfig)?;
        let api_server = api::start(api_url);

        Ok(Self {
            api_server,
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
            api_server,
            tx_collector,
            bundler,
            executor,
        } = self;

        // collector -> bundler
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = mpsc::channel(512);
        // bundler -> executor
        let (action_tx, action_rx): (Sender<Action>, Receiver<Action>) = mpsc::channel(512);

        let api_task = tokio::spawn(api_server.await);
        let collector_task = tokio::spawn(tx_collector.run(event_tx));
        let bundler_task = tokio::spawn(bundler.run(event_rx, action_tx));
        let executor_task = tokio::spawn(executor.run(action_rx));

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
            o = bundler_task => {
                match o {
                    Ok(task_result) => report_exit("bundler", task_result.map_err(Error::BundlerError)),
                    Err(e) => report_exit("bundler", Err(Error::TaskError(e))),
                }
            }
            o = executor_task => {
                match o {
                    Ok(task_result) => {
                        report_exit("executor", task_result.map_err(Error::ExecutorError)),
                    },
                    Err(e) => report_exit("sequencer executor", Err(Error::TaskError(e))),
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
        assert!(dbg!(searcher).is_ok());
    }
}
