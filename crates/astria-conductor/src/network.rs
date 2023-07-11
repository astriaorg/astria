pub(crate) use astria_gossipnet::network::Event;
use astria_gossipnet::network::{
    Network,
    NetworkBuilder,
    Sha256Topic,
};
use color_eyre::eyre::{
    Context,
    Result,
};

const BLOCKS_TOPIC: &str = "blocks";

pub(crate) struct GossipNetwork(pub(crate) Network);

impl GossipNetwork {
    pub(crate) fn new(bootnodes: Vec<String>, libp2p_private_key: Option<String>) -> Result<Self> {
        let mut builder = NetworkBuilder::new().bootnodes(bootnodes);
        if let Some(libp2p_private_key) = libp2p_private_key {
            builder = builder
                .keypair_from_file(&libp2p_private_key)
                .wrap_err("failed to load libp2p private key")?;
        }
        let mut network = builder.build().wrap_err("failed to build gossip network")?;
        network.subscribe(&Sha256Topic::new(BLOCKS_TOPIC));
        Ok(Self(network))
    }
}
