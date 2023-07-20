use astria_sequencer::transaction::Unsigned as SequencerUnsignedTx;
use astria_sequencer_client::Client as SequencerClient;
use color_eyre::eyre::{
    self,
    Report,
    WrapErr as _,
};
use ethers::types::Transaction;
use futures::TryFutureExt as _;
use tokio::sync::broadcast::{
    self,
    Receiver,
    Sender,
};
use tracing::error;

use self::{
    bundler::Bundler,
    collector::TxCollector,
    executor::SequencerExecutor,
};
use crate::Config;
mod bundler;
mod collector;
mod executor;

// #[derive(Debug)]
pub struct Searcher {
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

        Ok(Self {
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
    pub async fn run(self) -> eyre::Result<()> {
        let Self {
            tx_collector,
            bundler,
            executor,
        } = self;

        // collector -> bundler
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = broadcast::channel(512);
        // bundler -> executor
        let (action_tx, action_rx): (Sender<Action>, Receiver<Action>) = broadcast::channel(512);

        let collector_task =
            tokio::spawn(tx_collector.run(event_tx)).map_err(|res| ("collector", res));
        let bundler_task =
            tokio::spawn(bundler.run(event_rx, action_tx)).map_err(|res| ("bundler", res));
        let executor_task = tokio::spawn(executor.run(action_rx)).map_err(|res| ("executor", res));

        // FIXME: rework this to ensure all tasks shut down gracefully
        match tokio::try_join!(collector_task, bundler_task, executor_task) {
            Ok((collector_res, bundler_res, executor_res)) => {
                report_err("collector", collector_res);
                report_err("bundler", bundler_res);
                report_err("executor", executor_res);
                Ok(())
            }
            Err((task_name, join_err)) => Err(Report::new(join_err)
                .wrap_err(format!("task `{task_name}` failed to run to completion"))),
        }
    }
}

fn report_err(task_name: &'static str, res: eyre::Result<()>) {
    if let Err(e) = res {
        error!(task.name = task_name, error.message = %e, error.cause_chain = ?e, "task returned with an error");
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
