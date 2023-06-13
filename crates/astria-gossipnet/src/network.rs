/// gossipnet implements a basic gossip network using libp2p.
/// It currently supports discovery via bootnodes, mDNS, and the kademlia DHT.
use std::{
    collections::hash_map::DefaultHasher,
    hash::{
        Hash,
        Hasher,
    },
    pin::Pin,
    str::FromStr,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

use color_eyre::eyre::{
    eyre,
    Result,
    WrapErr,
};
use futures::StreamExt;
pub use libp2p::gossipsub::Sha256Topic;
#[cfg(feature = "dht")]
use libp2p::kad::{
    record::{
        store::MemoryStore,
        Key,
    },
    {
        Kademlia,
        KademliaConfig,
        KademliaEvent,
        QueryResult,
    },
};
#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{
    core::upgrade::Version,
    gossipsub::{
        self,
        Message,
        MessageId,
        TopicHash,
    },
    identify,
    identity::Keypair,
    kad::{
        AddProviderError,
        Addresses,
        GetClosestPeersError,
        GetProvidersError,
        QueryId,
    },
    noise,
    ping,
    swarm::{
        NetworkBehaviour,
        Swarm,
        SwarmBuilder,
        SwarmEvent,
    },
    tcp,
    yamux,
    Multiaddr,
    PeerId,
    Transport,
};
use multiaddr::Protocol;
use tracing::{
    debug,
    info,
    warn,
};

#[derive(NetworkBehaviour)]
struct GossipnetBehaviour {
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    gossipsub: gossipsub::Behaviour,
    #[cfg(feature = "mdns")]
    mdns: mdns::tokio::Behaviour,
    #[cfg(feature = "dht")]
    kademlia: Kademlia<MemoryStore>, // TODO: use disk store
}

pub struct NetworkBuilder {
    bootnodes: Option<Vec<String>>,
    port: u16,
    // TODO: load key file or keypair
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
    local_peer_id: PeerId,
    swarm: Swarm<GossipnetBehaviour>,
    terminated: bool,
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

#[derive(Debug)]
pub enum Event {
    NewListenAddr(Multiaddr),
    Message(Message),
    #[cfg(feature = "mdns")]
    MdnsPeersConnected(Vec<PeerId>),
    #[cfg(feature = "mdns")]
    MdnsPeersDisconnected(Vec<PeerId>),
    PeerConnected(PeerId),
    PeerSubscribed(PeerId, TopicHash),
    #[cfg(feature = "dht")]
    FoundProviders(Option<Key>, Option<Vec<PeerId>>),
    #[cfg(feature = "dht")]
    GetProvidersError(GetProvidersError),
    #[cfg(feature = "dht")]
    Providing(Key),
    #[cfg(feature = "dht")]
    ProvideError(AddProviderError),
    #[cfg(feature = "dht")]
    RoutingUpdated(PeerId, Addresses),
    #[cfg(feature = "dht")]
    FoundClosestPeers(Vec<PeerId>),
    #[cfg(feature = "dht")]
    GetClosestPeersError(GetClosestPeersError),
}

impl futures::Stream for Network {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        while let Poll::Ready(maybe_event) = self.swarm.poll_next_unpin(cx) {
            let Some(event) = maybe_event else {
                self.terminated = true;
                return Poll::Ready(None);
            };

            match event {
                // Swarm events
                SwarmEvent::NewListenAddr {
                    address, ..
                } => {
                    debug!("Local node is listening on {address}");
                    let maddr_str = format!("{}/p2p/{}", address, self.local_peer_id);
                    let Ok(multiaddr) = Multiaddr::from_str(&maddr_str) else {
                                        warn!("failed to parse multiaddr: {maddr_str}");
                                        continue;
                                    };

                    self.multiaddrs.push(multiaddr);
                    return Poll::Ready(Some(Event::NewListenAddr(address)));
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint: _,
                    num_established,
                    concurrent_dial_errors: _,
                    established_in: _,
                } => {
                    debug!(
                        "Connection with {peer_id} established (total: {num_established})",
                        peer_id = peer_id,
                        num_established = num_established,
                    );
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);

                    return Poll::Ready(Some(Event::PeerConnected(peer_id)));
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Identify(
                    libp2p::identify::Event::Received {
                        peer_id,
                        info,
                    },
                )) => {
                    debug!(
                        "Received identify event from {peer_id:?} with info: {info:?}",
                        peer_id = peer_id,
                        info = info,
                    );
                    for addr in info.listen_addrs {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr);
                    }
                }

                // DHT events
                #[cfg(feature = "dht")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Kademlia(
                    KademliaEvent::RoutingUpdated {
                        peer,
                        addresses,
                        old_peer,
                        ..
                    },
                )) => {
                    debug!(
                        "Routing table updated. Peer: {peer:?}, Addresses: {addresses:?}, Old \
                         peer: {old_peer:?}",
                        peer = peer,
                        addresses = addresses,
                        old_peer = old_peer,
                    );
                    return Poll::Ready(Some(Event::RoutingUpdated(peer, addresses)));
                }
                #[cfg(feature = "dht")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Kademlia(
                    KademliaEvent::RoutablePeer {
                        peer,
                        address,
                        ..
                    },
                )) => {
                    debug!(
                        "Routable peer: {peer:?}, Address: {address:?}",
                        peer = peer,
                        address = address,
                    );
                }
                #[cfg(feature = "dht")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Kademlia(
                    KademliaEvent::OutboundQueryProgressed {
                        id,
                        result,
                        ..
                    },
                )) => match result {
                    QueryResult::GetClosestPeers(res) => match res {
                        Ok(res) => {
                            debug!("found {} peers for query id {:?}", res.peers.len(), id,);
                            return Poll::Ready(Some(Event::FoundClosestPeers(res.peers)));
                        }
                        Err(e) => {
                            debug!("failed to find peers for {:?}: {}", id, e);
                            return Poll::Ready(Some(Event::GetClosestPeersError(e)));
                        }
                    },
                    QueryResult::Bootstrap(bootstrap) => {
                        if bootstrap.is_err() {
                            warn!(error = ?bootstrap.err(), "failed to bootstrap {:?}", id);
                            continue;
                        }

                        debug!("bootstrapping ok");
                    }
                    _ => {
                        debug!("query result for {:?}: {:?}", id, result);
                    }
                },

                // mDNS events
                #[cfg(feature = "mdns")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Mdns(mdns::Event::Discovered(
                    list,
                ))) => {
                    let peers = Vec::with_capacity(list.len());
                    for (peer_id, _multiaddr) in list {
                        debug!("mDNS discovered a new peer: {peer_id}");
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .add_explicit_peer(&peer_id);
                    }
                    return Poll::Ready(Some(Event::MdnsPeersConnected(peers)));
                }
                #[cfg(feature = "mdns")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Mdns(mdns::Event::Expired(
                    list,
                ))) => {
                    let peers = Vec::with_capacity(list.len());
                    for (peer_id, _multiaddr) in list {
                        debug!("mDNS discover peer has expired: {peer_id}");
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                    }
                    return Poll::Ready(Some(Event::MdnsPeersDisconnected(peers)));
                }

                // Gossipsub events
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    },
                )) => {
                    debug!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    );
                    return Poll::Ready(Some(Event::Message(message)));
                }
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Gossipsub(
                    gossipsub::Event::Subscribed {
                        peer_id,
                        topic,
                    },
                )) => {
                    debug!(
                        "Peer {peer_id} subscribed to topic: {topic:?}",
                        peer_id = peer_id,
                        topic = topic,
                    );
                    return Poll::Ready(Some(Event::PeerSubscribed(peer_id, topic)));
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Ping(_)) => {
                    // ignore for now
                }

                _ => {
                    debug!("unhandled swarm event: {:?}", event);
                }
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use futures::{
        channel::oneshot,
        join,
    };
    use tokio::{
        select,
        sync::watch,
    };

    use super::*;

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
    #[tokio::test]
    async fn test_dht_discovery() {
        // closed when task stops
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
