use std::{
    pin::Pin,
    str::FromStr,
    task::{
        Context,
        Poll,
    },
};

use color_eyre::eyre::{
    eyre,
    Result,
};
use futures::StreamExt;
use libp2p::{
    gossipsub::{
        self,
        Message,
        TopicHash,
    },
    kad::{
        Addresses,
        KademliaEvent,
        QueryResult,
    },
    mdns,
    swarm::SwarmEvent,
    Multiaddr,
    PeerId,
};
use tracing::debug;

use crate::network::{
    GossipnetBehaviourEvent,
    Network,
};

#[derive(Debug)]
pub enum Event {
    // Swarm events
    NewListenAddr(Multiaddr),

    // mDNS events
    MdnsPeersConnected(Vec<PeerId>),
    MdnsPeersDisconnected(Vec<PeerId>),

    // Gossipsub events
    GossipsubPeerConnected(PeerId),
    GossipsubPeerSubscribed(PeerId, TopicHash),
    GossipsubMessage(Message),

    // Kademlia events
    RoutingUpdated(PeerId, Addresses),
}

impl futures::Stream for Network {
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        while let Poll::Ready(maybe_event) = self.swarm.poll_next_unpin(cx) {
            let Some(event) = maybe_event else {
                return Poll::Ready(None);
            };

            match event {
                SwarmEvent::NewListenAddr {
                    address, ..
                } => {
                    debug!(address = ?address, "new listening address");
                    let maddr_str = format!("{}/p2p/{}", address, self.local_peer_id);
                    let Ok(multiaddr) = Multiaddr::from_str(&maddr_str) else {
                        return Poll::Ready(Some(Err(eyre!(
                            "failed to parse multiaddr: {:?}",
                            maddr_str
                        ))));
                    };

                    self.multiaddrs.push(multiaddr);
                    return Poll::Ready(Some(Ok(Event::NewListenAddr(address))));
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

                    return Poll::Ready(Some(Ok(Event::GossipsubPeerConnected(peer_id))));
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

                    if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
                        for addr in info.listen_addrs {
                            kademlia.add_address(&peer_id, addr);
                        }
                    }
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Kademlia(event)) => {
                    match self.handle_kademlia_event(event) {
                        Some(res) => return Poll::Ready(Some(res)),
                        None => continue,
                    }
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Mdns(event)) => {
                    match self.handle_mdns_event(event) {
                        Some(res) => return Poll::Ready(Some(res)),
                        None => continue,
                    }
                }

                SwarmEvent::Behaviour(GossipnetBehaviourEvent::Gossipsub(event)) => {
                    match self.handle_gossipsub_event(event) {
                        Some(res) => return Poll::Ready(Some(res)),
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
    fn handle_gossipsub_event(&mut self, event: gossipsub::Event) -> Option<Result<Event>> {
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
                Some(Ok(Event::GossipsubMessage(message)))
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
                Some(Ok(Event::GossipsubPeerSubscribed(peer_id, topic)))
            }
            _ => {
                debug!(event = ?event, "unhandled gossipsub event");
                None
            }
        }
    }

    fn handle_mdns_event(&mut self, event: mdns::Event) -> Option<Result<Event>> {
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
                Some(Ok(Event::MdnsPeersConnected(peers)))
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
                Some(Ok(Event::MdnsPeersDisconnected(peers)))
            }
        }
    }

    fn handle_kademlia_event(&self, event: KademliaEvent) -> Option<Result<Event>> {
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
                Some(Ok(Event::RoutingUpdated(peer, addresses)))
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
                QueryResult::Bootstrap(bootstrap) => {
                    if bootstrap.is_err() {
                        return Some(Err(eyre!(
                            "failed to bootstrap with query id {:?}: {:?}",
                            id,
                            bootstrap.err()
                        )));
                    }

                    debug!("bootstrapping ok");
                    None
                }
                other => {
                    debug!(query_id = ?id, result = ?other, "got query result");
                    None
                }
            },
            other => {
                debug!(event = ?other, "unhandled kademlia event");
                None
            }
        }
    }
}
