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
    eyre,
    Result,
    WrapErr,
};
pub use libp2p::gossipsub::Sha256Topic;
#[cfg(feature = "dht")]
use libp2p::kad::{
    record::store::MemoryStore,
    {
        Kademlia,
        KademliaConfig,
    },
};
#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{
    core::upgrade::Version,
    gossipsub::{
        self,
        MessageId,
    },
    identify,
    identity::Keypair,
    kad::QueryId,
    noise,
    ping,
    swarm::{
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
use multiaddr::Protocol;
use tracing::info;

pub use crate::stream::Event;

#[derive(NetworkBehaviour)]
pub(crate) struct GossipnetBehaviour {
    pub(crate) ping: ping::Behaviour,
    pub(crate) identify: identify::Behaviour,
    pub(crate) gossipsub: gossipsub::Behaviour,
    #[cfg(feature = "mdns")]
    pub(crate) mdns: mdns::tokio::Behaviour,
    #[cfg(feature = "dht")]
    pub(crate) kademlia: Kademlia<MemoryStore>, // TODO: use disk store
}

pub struct NetworkBuilder {
    bootnodes: Option<Vec<String>>,
    port: u16,
    keypair: Option<Keypair>,
}

impl NetworkBuilder {
    pub fn new() -> Self {
        Self {
            bootnodes: None,
            port: 0, // random port
            keypair: None,
        }
    }

    pub fn bootnodes(mut self, bootnodes: Vec<String>) -> Self {
        self.bootnodes = Some(bootnodes);
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn keypair(mut self, keypair: Keypair) -> Self {
        // TODO: load/store keypair from disk
        self.keypair = Some(keypair);
        self
    }

    pub fn build(self) -> Result<Network> {
        Network::new(
            self.keypair.unwrap_or(Keypair::generate_ed25519()),
            self.bootnodes,
            self.port,
        )
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Network {
    pub multiaddrs: Vec<Multiaddr>,
    pub(crate) local_peer_id: PeerId,
    pub(crate) swarm: Swarm<GossipnetBehaviour>,
    pub(crate) terminated: bool,
}

impl Network {
    pub fn new(local_key: Keypair, bootnodes: Option<Vec<String>>, port: u16) -> Result<Self> {
        let public_key = local_key.public();
        let local_peer_id = PeerId::from(public_key.clone());
        info!("local peer id: {local_peer_id:?}");

        let transport = tcp::tokio::Transport::default()
            .upgrade(Version::V1Lazy)
            .authenticate(noise::Config::new(&local_key)?)
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
            .map_err(|e| eyre!("failed to build gossipsub config: {}", e))?;

        // build a gossipsub network behaviour
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key),
            gossipsub_config,
        )
        .map_err(|e| eyre!("failed to create gossipsub behaviour: {}", e))?;

        let mut swarm = {
            #[cfg(feature = "dht")]
            let kademlia = {
                let mut cfg = KademliaConfig::default();
                cfg.set_query_timeout(Duration::from_secs(5 * 60));
                let store = MemoryStore::new(local_peer_id);
                Kademlia::with_config(local_peer_id, store, cfg)
            };

            #[cfg(feature = "mdns")]
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
            let behaviour = GossipnetBehaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    "gossipnet/0.1.0".into(),
                    public_key,
                )),
                gossipsub,
                #[cfg(feature = "mdns")]
                mdns,
                ping: ping::Behaviour::default(),
                #[cfg(feature = "dht")]
                kademlia,
            };
            SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
        };

        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
        swarm.listen_on(listen_addr.parse()?)?;

        if let Some(addrs) = bootnodes {
            addrs.iter().try_for_each(|addr| -> Result<_> {
                let mut maddr: Multiaddr = addr.parse()?;
                swarm.dial(maddr.clone())?;

                let Some(peer_id) = maddr.pop() else {
                    return Err(eyre!("failed to parse peer id from addr: {}", addr));
                };

                match peer_id {
                    Protocol::P2p(peer_id) => {
                        let peer_id = match PeerId::from_multihash(peer_id) {
                            Ok(peer_id) => peer_id,
                            Err(e) => {
                                return Err(eyre!("failed to parse peer id from addr: {:?}", e));
                            }
                        };

                        #[cfg(feature = "dht")]
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, maddr);
                    }
                    _ => {
                        return Err(eyre!("failed to parse peer id from addr: {}", addr));
                    }
                }

                Ok(())
            })?;
        }

        Ok(Network {
            multiaddrs: vec![],
            local_peer_id,
            swarm,
            terminated: false,
        })
    }

    #[cfg(feature = "dht")]
    pub async fn bootstrap(&mut self) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map(|_| ())
            .map_err(|e| eyre!(e))
    }

    #[cfg(feature = "dht")]
    pub async fn random_walk(&mut self) -> QueryId {
        self.swarm
            .behaviour_mut()
            .kademlia
            .get_closest_peers(PeerId::random())
    }

    pub async fn publish(&mut self, message: Vec<u8>, topic: Sha256Topic) -> Result<MessageId> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, message)
            .wrap_err("failed to publish message")
    }

    pub fn subscribe(&mut self, topic: &Sha256Topic) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(topic)
            .unwrap();
    }

    pub fn unsubscribe(&mut self, topic: &Sha256Topic) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(topic)
            .unwrap();
    }

    pub fn peer_count(&self) -> usize {
        self.swarm.network_info().num_peers()
    }
}

#[cfg(test)]
mod test {
    use futures::{
        channel::oneshot,
        join,
        StreamExt,
    };
    use tokio::{
        select,
        sync::watch,
    };

    use super::*;
    use crate::stream::Event;

    const TEST_TOPIC: &str = "test";

    #[tokio::test]
    async fn test_gossip_two_nodes() {
        let (bootnode_tx, bootnode_rx) = oneshot::channel();
        let (alice_tx, mut alice_rx) = oneshot::channel();

        let msg_a = b"hello world".to_vec();
        let recv_msg_a = msg_a.clone();
        let msg_b = b"i am responding".to_vec();
        let recv_msg_b = msg_b.clone();

        let alice_handle = tokio::task::spawn(async move {
            let topic = Sha256Topic::new(TEST_TOPIC);

            let mut alice = Network::new(Keypair::generate_ed25519(), None, 0).unwrap();
            alice.subscribe(&topic);

            let Some(event) = alice.next().await else {
                panic!("expected stream event");
            };

            match event {
                Event::NewListenAddr(addr) => {
                    println!("Alice listening on {:?}", addr);
                    let maddrs = &alice.multiaddrs;
                    assert_eq!(maddrs.len(), 1);
                    let maddr = maddrs[0].clone();
                    println!("Alice's maddr: {:?}", maddr);
                    bootnode_tx.send(maddr).unwrap();
                }
                _ => panic!("unexpected event"),
            };

            loop {
                let Some(event) = alice.next().await else {
                    break;
                };

                match event {
                    Event::PeerConnected(peer_id) => {
                        println!("Alice connected to {:?}", peer_id);
                    }
                    Event::PeerSubscribed(peer_id, topic_hash) => {
                        println!("Remote peer {:?} subscribed to {:?}", peer_id, topic_hash);
                        alice.publish(msg_a.clone(), topic.clone()).await.unwrap();
                    }
                    Event::Message(msg) => {
                        println!("Alice got message: {:?}", msg);
                        assert_eq!(msg.data, recv_msg_b);
                        alice_tx.send(()).unwrap();
                        return;
                    }
                    _ => {}
                }
            }
        });

        let bob_handle = tokio::task::spawn(async move {
            let topic = Sha256Topic::new(TEST_TOPIC);

            let bootnode = bootnode_rx.await.unwrap();
            let mut bob = Network::new(
                Keypair::generate_ed25519(),
                Some(vec![bootnode.to_string()]),
                0,
            )
            .unwrap();
            bob.subscribe(&topic);

            loop {
                select! {
                    event = bob.next() => {
                        let Some(event) = event else {
                            continue;
                        };

                        match event {
                            Event::PeerConnected(peer_id) => {
                                println!("Bob connected to {:?}", peer_id);
                            }
                            Event::Message(msg) => {
                                println!("Bob got message: {:?}", msg);
                                assert_eq!(msg.data, recv_msg_a);
                                bob.publish(msg_b.clone(), topic.clone()).await.unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ = &mut alice_rx => {
                        return;
                    }
                }
            }
        });

        let (res_a, res_b) = join!(alice_handle, bob_handle);
        res_a.unwrap();
        res_b.unwrap();
    }

    // this test starts 3 nodes; Alice, Bob and Charlie.
    // it connects Bob and Charlie to Alice directly, then tests that Bob and Charlie can
    // discover each other via the DHT.
    // the test completes when Charlie's peer count is 2 (Alice and Bob).
    // when this happens, he sends a value on his notification channel and returns from his task,
    // causing the Alice and Bob tasks to also return.
    #[tokio::test]
    async fn test_dht_discovery() {
        // notification sent when task stops
        let (charlie_tx, mut charlie_rx) = oneshot::channel();
        let (bob_tx, mut bob_rx) = oneshot::channel();

        // for sending the bootnode (Alice's) address to Bob and Charlie
        let (bootnode_tx, mut bootnode_rx) = watch::channel(None);
        let mut charlie_bootnode_rx = bootnode_rx.clone();

        // Charlie's local node key and peer id
        let charlie_local_key = Keypair::generate_ed25519();

        let alice_handle = tokio::task::spawn(async move {
            let mut alice = Network::new(Keypair::generate_ed25519(), None, 9000).unwrap();

            let Some(event) = alice.next().await else {
                panic!("expected stream event");
            };

            match event {
                Event::NewListenAddr(addr) => {
                    println!("Alice listening on {:?}", addr);
                    let maddrs = &alice.multiaddrs;
                    assert_eq!(maddrs.len(), 1);
                    let maddr = maddrs[0].clone();
                    println!("Alice's maddr: {:?}", maddr);
                    bootnode_tx.send(Some(maddr)).unwrap();
                }
                _ => panic!("unexpected event"),
            };

            loop {
                select! {
                    event = alice.next() => {
                        let Some(event) = event else {
                            continue;
                        };

                        match event {
                            Event::PeerConnected(peer_id) => {
                                println!("Alice connected to {:?}", peer_id);
                            }
                            Event::RoutingUpdated(peer_id, addresses) => {
                                println!("Alice's routing table updated by {:?} with addresses {:?}", peer_id, addresses);
                            }
                            _ => {}
                        }
                    }
                    _ = &mut bob_rx => {
                        return;
                    }
                }
            }
        });

        let bob_handle = tokio::task::spawn(async move {
            bootnode_rx.changed().await.unwrap();
            let bootnode = bootnode_rx.borrow().to_owned().unwrap();
            let mut bob = Network::new(
                Keypair::generate_ed25519(),
                Some(vec![bootnode.to_string()]),
                9001,
            )
            .unwrap();

            loop {
                select! {
                    event = bob.next() => {
                        let Some(event) = event else {
                            continue;
                        };

                        match event {
                            Event::PeerConnected(peer_id) => {
                                println!("Bob connected to {:?}", peer_id);
                            }
                            Event::FoundClosestPeers(peers) => {
                                println!("Bob found closest peers {:?}", peers);
                            }
                            Event::RoutingUpdated(peer_id, addresses) => {
                                println!("Bob's routing table updated by {:?} with addresses {:?}", peer_id, addresses);
                                bob.random_walk().await;
                            }
                            _ => {}
                        }
                    }
                    _ = &mut charlie_rx => {
                        bob_tx.send(()).unwrap();
                        return;
                    }
                }
            }
        });

        let charlie_handle = tokio::task::spawn(async move {
            charlie_bootnode_rx.changed().await.unwrap();
            let bootnode = charlie_bootnode_rx.borrow().to_owned().unwrap();
            let mut charlie =
                Network::new(charlie_local_key, Some(vec![bootnode.to_string()]), 9002).unwrap();

            loop {
                let Some(event) = charlie.next().await else {
                        break;
                    };

                match event {
                    Event::PeerConnected(peer_id) => {
                        println!("Charlie connected to {:?}", peer_id);
                        if charlie.peer_count() == 1 {
                            charlie.random_walk().await;
                        }

                        if charlie.peer_count() == 2 {
                            charlie_tx.send(()).unwrap();
                            return;
                        }
                    }
                    Event::FoundClosestPeers(peers) => {
                        println!("Charlie found closest peers {:?}", peers);
                    }
                    Event::ProvideError(e) => {
                        panic!("Charlie failed to provide: {:?}", e);
                    }
                    _ => {}
                }
            }
        });

        let (res_a, res_b, res_c) = join!(alice_handle, bob_handle, charlie_handle);
        res_a.unwrap();
        res_b.unwrap();
        res_c.unwrap();
    }
}
