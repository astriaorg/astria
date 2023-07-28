use std::net::SocketAddr;

use eyre::WrapErr as _;
use tokio::{
    sync::mpsc::unbounded_channel,
    task::JoinError,
};

use crate::{
    api,
    config::Config,
    network::GossipNetwork,
    relayer::Relayer,
};

pub struct SequencerRelayer {
    api_server: api::ApiServer,
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
        let (block_tx, block_rx) = unbounded_channel();
        let (gossipnet_info_tx, gossipnet_info_rx) = unbounded_channel();
        let gossip_net = GossipNetwork::new(&cfg, block_rx, gossipnet_info_rx)
            .wrap_err("failed to create gossip network")?;
        let relayer = Relayer::new(&cfg, block_tx).wrap_err("failed to create relayer")?;
        let state_rx = relayer.subscribe_to_state();
        let api_server = api::start(cfg.rpc_port, state_rx, gossipnet_info_tx);
        Ok(Self {
            api_server,
            gossip_net,
            relayer,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    pub async fn run(self) {
        let Self {
            api_server,
            gossip_net,
            relayer,
        } = self;
        let gossip_task = tokio::spawn(gossip_net.run());

        // Wrap the API server in an async block so we can easily turn the result
        // of the future into an eyre report.
        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });
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
