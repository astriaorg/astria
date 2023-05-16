pub(crate) use astria_gossipnet::network::Event;
use astria_gossipnet::network::{
    Network,
    NetworkBuilder,
    Sha256Topic,
};
use color_eyre::eyre::Result;

const BLOCKS_TOPIC: &str = "blocks";

pub(crate) struct GossipNetwork(pub(crate) Network);

impl GossipNetwork {
    pub(crate) fn new(bootnodes: Vec<String>) -> Result<Self> {
        let mut network = NetworkBuilder::new().bootnodes(bootnodes).build()?;
        network.subscribe(&Sha256Topic::new(BLOCKS_TOPIC));
        Ok(Self(network))
    }
}
