use crate::{
    bundler::Bundler,
    collector::Collector,
    ds::{EthProvider, SequencerClient, StreamingClient},
    executor::Executor,
    Config,
};
use color_eyre::eyre::{self, Context};
use tokio::sync::watch;

pub struct SearcherStatus {
    active_providers: u64,
    sequencer_connected: bool
}

impl SearcherStatus {
    pub(crate) fn is_ready(&self) -> bool {
        (self.active_providers != 0) && self.sequencer_connected
    }
}

pub struct Searcher {
    providers: Vec<Box<dyn StreamingClient<Error = eyre::Error>>>,
    collector: Collector,
    bundler: Bundler,
    executor: Executor,
    status: watch::Sender<SearcherStatus>
}

impl Searcher {
    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<SearcherStatus> {
        self.status.subscribe()
    }

    /// Constructs a new Searcher service from config.
    pub(super) async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let eth_client = EthProvider::connect(&cfg.execution_url)
            .await
            .wrap_err("failed connecting to eth")?;

        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let rollup_chain_id = cfg.chain_id.clone();
        let (status, _) = watch::channel(Status::default());


    }

    pub async fn start() {
        ds::EthersProvider

        self.status.send_modify(|status| {
            status.sequencer_connected = true;
        });
    }
}
