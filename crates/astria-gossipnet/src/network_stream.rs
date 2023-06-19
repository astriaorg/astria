use std::{
    pin::Pin,
    str::FromStr,
    task::{
        Context,
        Poll,
    },
};

use futures::StreamExt;
#[cfg(feature = "dht")]
use libp2p::kad::{
    record::Key,
    {
        KademliaEvent,
        QueryResult,
    },
};
#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{
    gossipsub::{
        self,
        Message,
        TopicHash,
    },
    kad::{
        AddProviderError,
        Addresses,
        GetClosestPeersError,
        GetProvidersError,
    },
    swarm::SwarmEvent,
    Multiaddr,
    PeerId,
};
use tracing::{
    debug,
    warn,
};

use crate::network::{
    GossipnetBehaviourEvent,
    Network,
};

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
                    num_established,
                    ..
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
