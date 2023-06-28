use astria_gossipnet::network::{
    Network,
    NetworkBuilder,
    Sha256Topic,
};
use eyre::Result;
use futures::StreamExt;
use tokio::{
    select,
    sync::watch,
    task::JoinHandle,
};
use tracing::{
    debug,
    warn,
};

use crate::sequencer_block::SequencerBlock;

const BLOCKS_TOPIC: &str = "blocks";

pub struct GossipNetwork {
    network: Network,
    block_rx: watch::Receiver<Option<SequencerBlock>>,
}

impl GossipNetwork {
    pub fn new(p2p_port: u16, block_rx: watch::Receiver<Option<SequencerBlock>>) -> Result<Self> {
        let network = NetworkBuilder::new().port(p2p_port).build()?;
        Ok(Self {
            network,
            block_rx,
        })
    }

    pub fn run(mut self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                select! {
                    res = self.block_rx.changed() => {
                        if res.is_err() {
                            warn!("block_rx channel closed");
                            break;
                        }

                        let Some(block) = self.block_rx.borrow().clone() else {
                            panic!("block_rx should not receive None")
                        };

                        match self.publish(&block).await {
                            Ok(()) => debug!(block_hash = ?block.block_hash, "published block to network"),
                            Err(e) => warn!(?e, "failed to publish block to network"),
                        };
                    },
                    event = self.network.next() => {
                        if let Some(event) = event {
                            debug!(?event, "got event from network");
                        }
                    },
                }
            }
        })
    }

    async fn publish(&mut self, block: &SequencerBlock) -> Result<()> {
        self.network
            .publish(block.to_bytes()?, Sha256Topic::new(BLOCKS_TOPIC))
            .await?;
        Ok(())
    }
}
