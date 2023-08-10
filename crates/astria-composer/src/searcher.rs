use crate::{
    bundler::Bundler,
    collector::Collector,
    ds::{EthProvider, RollupChainId, SequencerClient, StreamingClient},
    executor::Executor,
    Config,
};
use color_eyre::eyre::{self, Context};
use tokio::sync::watch;

#[derive(Default)]
pub struct SearcherStatus {
    active_providers: u64,
    sequencer_connected: bool,
}

impl SearcherStatus {
    pub(crate) fn is_ready(&self) -> bool {
        (self.active_providers != 0) && self.sequencer_connected
    }
}

pub struct Searcher {
    status: watch::Sender<SearcherStatus>,
}

impl Searcher {
    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<SearcherStatus> {
        self.status.subscribe()
    }

    // FIXME: How do we start up multiple providers if we are reading from Config?
    async fn setup_providers_from_config(
        cfg: &Config,
    ) -> Result<Vec<(Box<dyn StreamingClient<Error = eyre::Error>>, RollupChainId)>, eyre::Error>
    {
        let eth_client = EthProvider::connect(&cfg.execution_url)
            .await
            .wrap_err("failed connecting to eth")?;

        Ok(vec![(Box::new(eth_client), "1".to_string())])
    }

    /// Constructs a new Searcher service from config.
    pub(super) async fn from_config(cfg: &Config) -> Result<Self, eyre::Error> {
        let (collector, collector_recv_channel) = Collector::new();

        let mut status = SearcherStatus::default();
        for (provider, chain_id) in Self::setup_providers_from_config(cfg).await? {
            collector.add_provider(provider, chain_id).await?;
            status.active_providers += 1;
        }

        let (mut bundler, bundler_recv_channel) = Bundler::new(collector_recv_channel);
        let mut executor = Executor::new(&cfg.sequencer_url, bundler_recv_channel).await?;
        // Set status flag to true since sequencer has started successfully if this code is executed
        status.sequencer_connected = true;

        let (status, _) = watch::channel(status);
        tokio::task::spawn(async move {
            bundler.start();
        });

        tokio::task::spawn(async move{
            executor.start();
        });

        Ok(Self {
            status
        })
    }
}
