use astria_gossipnet::network::{
    Network,
    NetworkBuilder,
    Sha256Topic,
};
use eyre::{
    Result,
    WrapErr as _,
};
use futures::StreamExt;
use tokio::{
    select,
    sync::{
        mpsc::UnboundedReceiver,
        oneshot,
    },
};
use tracing::{
    debug,
    warn,
};

use crate::{
    config::Config,
    types::SequencerBlockData,
};

const BLOCKS_TOPIC: &str = "blocks";
fn blocks_topic() -> Sha256Topic {
    Sha256Topic::new(BLOCKS_TOPIC)
}

pub struct GossipNetwork {
    network: Network,
    block_rx: UnboundedReceiver<SequencerBlockData>,
    info_query_rx: UnboundedReceiver<InfoQuery>,
}

impl GossipNetwork {
    pub(crate) fn new(
        cfg: &Config,
        block_rx: UnboundedReceiver<SequencerBlockData>,
        info_query_rx: UnboundedReceiver<InfoQuery>,
    ) -> Result<Self> {
        let mut builder = NetworkBuilder::new().port(cfg.p2p_port);

        if let Some(bootnodes) = &cfg.bootnodes {
            builder = builder.bootnodes(bootnodes.clone());
        }

        if let Some(libp2p_private_key) = &cfg.libp2p_private_key {
            builder = builder
                .keypair_from_file(libp2p_private_key)
                .wrap_err("failed to load libp2p private key")?;
        }

        let network = builder.build().wrap_err("failed to build gossip network")?;
        Ok(Self {
            network,
            block_rx,
            info_query_rx,
        })
    }

    /// Runs the gossip network.
    ///
    /// # Errors
    ///
    /// `GossipNetwork::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        loop {
            select! {
                block = self.block_rx.recv() => {
                    if let Some(block) = block {
                        match self.publish(&block).await {
                            Ok(()) => debug!(block_hash = ?block.block_hash, "published block to network"),
                            Err(e) => {
                                if e.root_cause().to_string().contains("InsufficientPeers") {
                                    debug!(?e, "failed to publish block to network");
                                    continue;
                                }
                                warn!(?e, "failed to publish block to network")
                            },
                        };
                    }
                },
                Some(query) = self.info_query_rx.recv() => {
                    match query {
                        InfoQuery::NumberOfPeers(tx) => {
                            let n = self.network.num_subscribed(&blocks_topic());
                            if tx.send(n).is_err() {
                                warn!("oneshot sender to respond to number of peers info query dropped before a value could be sent");
                            }
                        }
                    }
                }
                event = self.network.next() => {
                    if let Some(event) = event {
                        debug!(?event, "got event from network");
                    }
                },
            }
        }
        // Return Ok to make the types align (see the method's doc comment why this is necessary).
        // Allow unreachable code to quiet warnings
        #[allow(unreachable_code)]
        Ok(())
    }

    async fn publish(&mut self, block: &SequencerBlockData) -> Result<()> {
        self.network
            .publish(block.to_bytes()?, blocks_topic())
            .await?;
        Ok(())
    }
}

pub(crate) enum InfoQuery {
    NumberOfPeers(oneshot::Sender<usize>),
}
