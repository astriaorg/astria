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
                SwarmEvent::NewListenAddr {
                    address, ..
                } => {
                    debug!(address = ?address, "new listening address");
                    let maddr_str = format!("{}/p2p/{}", address, self.local_peer_id);
                    let Ok(multiaddr) = Multiaddr::from_str(&maddr_str) else {
                        warn!(multiaddr = ?maddr_str, "failed to parse multiaddr");
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
                        peer_id = ?peer_id,
                        num_established = ?num_established,
                        "connection established",
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
                        peer_id = ?peer_id,
                        info = ?info,
                        "received Identify info",
                    );
                    for addr in info.listen_addrs {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr);
                    }
                }

                #[cfg(feature = "dht")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Kademlia(event)) => {
                    match self.handle_kademlia_event(event) {
                        Some(event) => return Poll::Ready(Some(event)),
                        None => continue,
                    }
                }

                #[cfg(feature = "mdns")]
                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Mdns(event)) => {
                    match self.handle_mdns_event(event) {
                        Some(event) => return Poll::Ready(Some(event)),
                        None => continue,
                    }
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Gossipsub(event)) => {
                    match self.handle_gossipsub_event(event) {
                        Some(event) => return Poll::Ready(Some(event)),
                        None => continue,
                    }
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Ping(_)) => {
                    // ignore for now
                }

                _ => {
                    debug!(event = ?event, "unhandled swarm event");
                }
            }
        }

        Poll::Pending
    }
}

impl Network {
    fn handle_gossipsub_event(&mut self, event: gossipsub::Event) -> Option<Event> {
        match event {
            gossipsub::Event::Message {
                propagation_source: peer_id,
                message_id: id,
                message,
            } => {
                debug!(
                    id = ?id,
                    peer_id = ?peer_id,
                    "received message from peer",
                );
                Some(Event::Message(message))
            }
            gossipsub::Event::Subscribed {
                peer_id,
                topic,
            } => {
                debug!(
                    peer_id = ?peer_id,
                    topic = ?topic,
                    "peer subscribed to topic",
                );
                Some(Event::PeerSubscribed(peer_id, topic))
            }
            _ => {
                debug!(event = ?event, "unhandled gossipsub event");
                None
            }
        }
    }

    #[cfg(feature = "mdns")]
    fn handle_mdns_event(&mut self, event: mdns::Event) -> Option<Event> {
        match event {
            mdns::Event::Discovered(list) => {
                let peers = Vec::with_capacity(list.len());
                for (peer_id, _multiaddr) in list {
                    debug!(
                        peer_id = ?peer_id,
                        "peer discovered via mDNS",
                    );
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                }
                Some(Event::MdnsPeersConnected(peers))
            }
            mdns::Event::Expired(list) => {
                let peers = Vec::with_capacity(list.len());
                for (peer_id, _multiaddr) in list {
                    debug!(
                        peer_id = ?peer_id,
                        "mDNS peer expired",
                    );
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
                Some(Event::MdnsPeersDisconnected(peers))
            }
        }
    }

    #[cfg(feature = "dht")]
    fn handle_kademlia_event(&self, event: KademliaEvent) -> Option<Event> {
        match event {
            KademliaEvent::RoutingUpdated {
                peer,
                addresses,
                old_peer,
                ..
            } => {
                debug!(
                    peer = ?peer,
                    addresses = ?addresses,
                    old_peer = ?old_peer,
                    "routing table updated",
                );
                Some(Event::RoutingUpdated(peer, addresses))
            }
            KademliaEvent::RoutablePeer {
                peer,
                address,
                ..
            } => {
                debug!(
                    peer = ?peer,
                    address = ?address,
                    "routable peer",
                );
                None
            }
            KademliaEvent::OutboundQueryProgressed {
                id,
                result,
                ..
            } => match result {
                QueryResult::GetClosestPeers(res) => match res {
                    Ok(res) => {
                        debug!(num_peers = res.peers.len(), query_id = ?id, "found closest peers");
                        Some(Event::FoundClosestPeers(res.peers))
                    }
                    Err(e) => {
                        debug!(query_id = ?id, error = ?e, "failed to find peers");
                        Some(Event::GetClosestPeersError(e))
                    }
                },
                QueryResult::Bootstrap(bootstrap) => {
                    if bootstrap.is_err() {
                        warn!(query_id = ?id, error = ?bootstrap.err(), "failed to bootstrap");
                    }

                    debug!("bootstrapping ok");
                    None
                }
                _ => {
                    debug!(query_id = ?id, result = ?result, "got query result");
                    None
                }
            },
            _ => {
                debug!(event = ?event, "unhandled kademlia event");
                None
            }
        }
    }
}
