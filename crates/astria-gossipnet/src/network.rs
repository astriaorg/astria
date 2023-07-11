/// gossipnet implements a basic gossip network using libp2p.
/// It currently supports discovery via bootnodes, mDNS, and the kademlia DHT.
use std::{
    collections::hash_map::DefaultHasher,
    hash::{
        Hash,
        Hasher,
    },
    time::Duration,
};

use color_eyre::eyre::{
    bail,
    eyre,
    Result,
    WrapErr,
};
use libp2p::{
    core::upgrade::Version,
    gossipsub::{
        self,
        MessageId,
    },
    identify,
    kad::{
        record::store::MemoryStore,
        Kademlia,
        KademliaConfig,
        QueryId,
    },
    mdns,
    noise,
    ping,
    swarm::{
        behaviour::toggle::Toggle,
        NetworkBehaviour,
        Swarm,
        SwarmBuilder,
    },
    tcp,
    yamux,
    Multiaddr,
    PeerId,
    Transport,
};
pub use libp2p::{
    gossipsub::Sha256Topic,
    identity::Keypair,
};
use multiaddr::Protocol;
use tracing::info;

pub use crate::network_stream::Event;

const GOSSIPNET_PROTOCOL_ID: &str = "gossipnet/0.1.0";

#[derive(NetworkBehaviour)]
pub(crate) struct GossipnetBehaviour {
    pub(crate) ping: ping::Behaviour,
    pub(crate) identify: identify::Behaviour,
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) mdns: Toggle<mdns::tokio::Behaviour>,
    pub(crate) kademlia: Toggle<Kademlia<MemoryStore>>, // TODO: use disk store
}

pub struct NetworkBuilder {
    bootnodes: Option<Vec<String>>,
    port: u16,
    keypair: Option<Keypair>,
    with_mdns: bool,
    with_kademlia: bool,
}

impl NetworkBuilder {
    pub fn new() -> Self {
        Self {
            bootnodes: None,
            port: 0, // random port
            keypair: None,
            with_mdns: true,
            with_kademlia: true,
        }
    }

    /// The bootnodes to initially connect to.
    pub fn bootnodes(mut self, bootnodes: Vec<String>) -> Self {
        self.bootnodes = Some(bootnodes);
        self
    }

    /// The port to listen on.
    /// If not provided, a random port will be chosen.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Specify the keypair to use from a file.
    /// The file should contain a hex-encoded 32-byte ed25519 secret key.
    pub fn keypair_from_file(mut self, path: &str) -> Result<Self> {
        let key_string = std::fs::read_to_string(path).wrap_err("failed to read keypair file")?;
        if key_string.len() < 64 {
            bail!("keypair file is too short");
        }

        let bytes =
            hex::decode(&key_string[0..64]).wrap_err("failed to decode keypair hex string")?;
        let keypair = Keypair::ed25519_from_bytes(bytes).wrap_err("failed to load keypair")?;
        self.keypair = Some(keypair);
        Ok(self)
    }

    /// The keypair to use for the node.
    /// If not provided, a new keypair will be generated.
    pub fn keypair(mut self, keypair: Keypair) -> Self {
        // TODO: load/store keypair from disk
        self.keypair = Some(keypair);
        self
    }

    /// Whether to enable mDNS discovery.
    pub fn with_mdns(mut self, with_mdns: bool) -> Self {
        self.with_mdns = with_mdns;
        self
    }

    /// Whether to enable kademlia DHT discovery.
    /// Note: the query timeout is set by default to 60 seconds.
    pub fn with_kademlia(mut self, with_kademlia: bool) -> Self {
        self.with_kademlia = with_kademlia;
        self
    }

    /// Builds the `Network`.
    /// Creates a TCP transport, builds a swarm, and connects to the bootnodes.
    pub fn build(self) -> Result<Network> {
        let keypair = self.keypair.unwrap_or(Keypair::generate_ed25519());
        let public_key = keypair.public();
        let local_peer_id = PeerId::from(public_key.clone());
        info!(local_peer_id = ?local_peer_id);

        let transport = tcp::tokio::Transport::default()
            .upgrade(Version::V1Lazy)
            .authenticate(noise::Config::new(&keypair)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // content-address message by using the hash of it as an ID
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict) // the default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages so that duplicates aren't propagated
            .build()
            .map_err(|e| eyre!("failed to create gossipsub behaviour: `{e}`"))?;

        // build a gossipsub network behaviour
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair),
            gossipsub_config,
        )
        .map_err(|e| eyre!("failed to create gossipsub behaviour: {}", e))?;

        let mut swarm = {
            let kademlia = if self.with_kademlia {
                let mut cfg = KademliaConfig::default();
                cfg.set_query_timeout(Duration::from_secs(60));
                let store = MemoryStore::new(local_peer_id);
                Some(Kademlia::with_config(local_peer_id, store, cfg))
            } else {
                None
            };

            let mdns = if self.with_mdns {
                Some(mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    local_peer_id,
                )?)
            } else {
                None
            };

            let behaviour = GossipnetBehaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    GOSSIPNET_PROTOCOL_ID.into(),
                    public_key,
                )),
                gossipsub,
                ping: ping::Behaviour::default(),
                mdns: mdns.into(),
                kademlia: kademlia.into(),
            };
            SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
        };

        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", self.port);
        swarm
            .listen_on(
                listen_addr
                    .parse()
                    .wrap_err("failed to parse listen addr")?,
            )
            .wrap_err("swarm failed to listen on address")?;

        if let Some(addrs) = self.bootnodes {
            addrs.iter().try_for_each(|addr| -> Result<_> {
                let mut maddr: Multiaddr = addr
                    .parse()
                    .wrap_err("failed to parse bootnode multiaddress")?;
                swarm
                    .dial(maddr.clone())
                    .wrap_err("failed to dial bootnode")?;

                let Some(maybe_peer_id) = maddr.pop() else {
                    bail!("failed to parse peer id from addr: `{addr}`");
                };

                match maybe_peer_id {
                    Protocol::P2p(peer_id) => {
                        let peer_id = PeerId::from_multihash(peer_id)
                            .map_err(|e| eyre!("failed to parse peer id from addr: {:?}", e))?;
                        if let Some(kademlia) = swarm.behaviour_mut().kademlia.as_mut() {
                            kademlia.add_address(&peer_id, maddr);
                        }
                    }
                    other => {
                        bail!(
                            "failed to parse peer id from addr {}, received {}",
                            addr,
                            other
                        );
                    }
                }

                Ok(())
            })?;
        }

        Ok(Network {
            multiaddrs: vec![],
            local_peer_id,
            swarm,
        })
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The `Network` struct represents a network with gossipsub enabled.
/// It is able to publish and subscribe to gossipsub topics.
/// It additionally has the `ping` and `identify` protocols enabled.
///
/// If kademlia is enabled, it is also able to discover peers via random walk.
pub struct Network {
    pub(crate) multiaddrs: Vec<Multiaddr>,
    pub(crate) local_peer_id: PeerId,
    pub(crate) swarm: Swarm<GossipnetBehaviour>,
}

impl Network {
    /// Returns the external listening addresses of the peer as received from
    /// `SwarmEvent::NewListenAddr`.
    pub fn multiaddrs(&self) -> Vec<Multiaddr> {
        self.multiaddrs.clone()
    }

    /// Bootstraps the node to join the DHT.
    /// see https://docs.rs/libp2p-kad/latest/libp2p_kad/struct.Kademlia.html#method.bootstrap
    pub async fn bootstrap(&mut self) -> Result<()> {
        let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() else {
            bail!("kademlia is not enabled")
        };
        kademlia
            .bootstrap()
            .map(|_| ())
            .wrap_err("failed to bootstrap kademlia")
    }

    /// Attempts to discover new peers via DHT random walk.
    /// It does a DHT query for the closest peers to a random peer ID.
    ///
    /// Since the `identify` protocol is enabled, it allows us to discover the
    /// listen addresses of new peers, which are added to the routing table
    /// when `libp2p::identify::Event::Received` is handled.
    pub async fn random_walk(&mut self) -> Result<QueryId> {
        if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
            Ok(kademlia.get_closest_peers(PeerId::random()))
        } else {
            bail!("kademlia is not enabled")
        }
    }

    /// Publishes a message to a gossipsub topic.
    pub async fn publish(&mut self, message: Vec<u8>, topic: Sha256Topic) -> Result<MessageId> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, message)
            .wrap_err("failed to publish message")
    }

    /// Subscribes to a gossipsub topic.
    pub fn subscribe(&mut self, topic: &Sha256Topic) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(topic)
            .unwrap();
    }

    /// Unsubscribes from a gossipsub topic.
    pub fn unsubscribe(&mut self, topic: &Sha256Topic) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(topic)
            .unwrap();
    }

    /// Returns the number of peers currently connected.
    pub fn num_peers(&self) -> usize {
        self.swarm.network_info().num_peers()
    }
}
