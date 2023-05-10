use eyre::Result;
use futures::StreamExt;
use gossipnet::network::{Network, NetworkBuilder, Sha256Topic};
use tokio::{select, sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::{debug, warn};

use crate::sequencer_block::SequencerBlock;

const BLOCKS_TOPIC: &str = "blocks";

pub struct GossipNetwork {
    network: Network,
    block_rx: UnboundedReceiver<SequencerBlock>,
}

impl GossipNetwork {
    pub fn new(p2p_port: u16, block_rx: UnboundedReceiver<SequencerBlock>) -> Result<Self> {
        let network = NetworkBuilder::new().port(p2p_port).build()?;
        Ok(Self { network, block_rx })
    }

    pub fn run(mut self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                select! {
                    block = self.block_rx.recv() => {
                        if let Some(block) = block {
                            match self.publish(&block).await {
                                Ok(()) => debug!(block_hash = ?block.block_hash, "published block to network"),
                                Err(e) => warn!(?e, "failed to publish block to network"),
                            };
                        }
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
