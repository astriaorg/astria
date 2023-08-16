use color_eyre::eyre::{self, Context};
use tokio::sync::{mpsc::unbounded_channel, watch};

use crate::{
    searcher::{collector::Collector, executor::Executor},
    Config,
};

use self::{
    clients::EthClient,
    data_structures::{streaming_client::StreamingClient, RollupChainId, RollupTxExt},
};

use crate::searcher::clients::sequencer_client::SequencerClient;

mod collector;
mod executor;

/// This module defines the common data structures that are used throughout the
/// Searcher implementation
mod data_structures;

/// These are the clients that the Searcher supports connecting to
/// including incoming connections from rollups and outgoing connections
/// to builders and the sequencer
mod clients;

// use crate::{
//     bundler::Bundler,
//     collector::Collector,
//     data_structures::{EthProvider, RollupChainId, SequencerClient, StreamingClient},
//     executor::Executor,
//     Config,
// };

#[derive(Default)]
pub struct Status {
    active_rollup_clients: u64,
    sequencer_connected: bool,
}

impl Status {
    pub(crate) fn is_ready(&self) -> bool {
        (self.active_rollup_clients != 0) && self.sequencer_connected
    }
}

pub(super) struct Searcher {
    collector: Collector,
    executor: Executor,
    status: watch::Sender<Status>,
}

impl Searcher {
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    pub(super) fn setup_searcher(cfg: &Config) -> Result<Self, eyre::Error> {
        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let (collector_sender, executor_receiver) = unbounded_channel();
        let collector = Collector::new(collector_sender);

        let sequencer_client = todo!();
        let executor = Executor::new(sequencer_client, executor_receiver)?;

        let (status, _) = watch::channel(Status::default());

        status.send_modify(|status| {
            status.sequencer_connected = true;
        });

        Ok(Self {
            collector,
            executor,
            status,
        })
    }

    pub(super) async fn setup_rollup_clients(
        cfg: &Config,
    ) -> Result<
        Vec<(
            Box<impl StreamingClient<Error = eyre::Error>>,
            RollupChainId,
        )>,
        eyre::Error,
    > {
        let eth_client = EthClient::connect(&cfg.execution_url)
            .await
            .wrap_err("failed connecting to eth")?;

        Ok(vec![(Box::new(eth_client), "1".to_string())])
    }

    pub(super) async fn run(
        &mut self,
        rollups: Vec<(
            Box<impl StreamingClient<Error = eyre::Error>>,
            RollupChainId,
        )>,
    ) -> Result<(), eyre::Error> {
        // Adding providers for all the providers that the searcher has to run
        // NOTE: each provider added will spawn its own tokio::task inside `add_provider`
        let mut new_clients = 0;
        for (provider, chain_id) in rollups {
            self.collector.add_provider(provider, chain_id).await?;
            new_clients += 1;
        }

        self.status.send_modify(|status| {
            status.active_rollup_clients += new_clients;
        });

        // Block until all the components have exited
        tokio::join!(self.executor.start());

        Ok(())
    }
}
