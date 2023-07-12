use eyre::WrapErr as _;
use futures::Future;
use tokio::task::JoinError;

use crate::{
    api,
    config::Config,
    network::GossipNetwork,
    relayer::Relayer,
};

pub struct SequencerRelayer {
    api_server: Box<dyn Future<Output = eyre::Result<()>> + Send + Unpin>,
    gossip_net: GossipNetwork,
    relayer: Relayer,
}

impl SequencerRelayer {
    /// Instantiates a new `SequencerRelayer`.
    ///
    /// # Errors
    ///
    /// Returns an error if constructing the gossip network or the relayer
    /// worked failed.
    pub fn new(cfg: Config) -> eyre::Result<Self> {
        let (block_tx, block_rx) = tokio::sync::mpsc::unbounded_channel();
        let gossip_net =
            GossipNetwork::new(&cfg, block_rx).wrap_err("failed to create gossip network")?;
        let relayer = Relayer::new(&cfg, block_tx).wrap_err("failed to create relayer")?;
        let state_rx = relayer.subscribe_to_state();
        let api_server = Box::new(api::start(cfg.rpc_port, state_rx));
        Ok(Self {
            api_server,
            gossip_net,
            relayer,
        })
    }

    pub async fn run(self) {
        let Self {
            api_server,
            gossip_net,
            relayer,
        } = self;
        let gossip_task = tokio::spawn(gossip_net.run());
        let api_task = tokio::spawn(api_server);
        let relayer_task = tokio::spawn(relayer.run());

        tokio::select!(
            o = gossip_task => report_exit("gossip network", o),
            o = api_task => report_exit("api server", o),
            o = relayer_task => report_exit("relayer worker", o),
        );
    }
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!(task = task_name, "task has exited"),
        Ok(Err(e)) => {
            tracing::error!(
                task = task_name,
                error.msg = %e,
                error.cause = ?e,
                "task exited with error"
            );
        }
        Err(e) => {
            tracing::error!(
                task = task_name,
                error.msg = %e,
                error.cause = ?e,
                "task failed to complete"
            );
        }
    }
}
