use astria_gossipnet::network::{
    Event,
    Keypair,
    NetworkBuilder,
    Sha256Topic,
};
use futures::{
    channel::oneshot,
    join,
    StreamExt,
};
use tokio::{
    select,
    sync::watch,
};

const TEST_TOPIC: &str = "test";

#[tokio::test]
async fn keypair_from_file() {
    let key_string = "241ccaff3cc8681385b2cf92a82b49ab2e9c6410f7b6191322867c4299de42d8";
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path();
    std::fs::write(path, key_string).unwrap();

    NetworkBuilder::new()
        .keypair_from_file(path)
        .unwrap()
        .with_mdns(false)
        .build()
        .unwrap();
}

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

        let mut alice = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .build()
            .unwrap();
        alice.subscribe(&topic);

        let Some(event) = alice.next().await else {
            panic!("expected stream event");
        };

        match event.unwrap() {
            Event::NewListenAddr(addr) => {
                println!("Alice listening on {:?}", addr);
                let maddrs = alice.multiaddrs();
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

            match event.unwrap() {
                Event::GossipsubPeerConnected(peer_id) => {
                    println!("Alice connected to {:?}", peer_id);
                }
                Event::GossipsubPeerSubscribed(peer_id, topic_hash) => {
                    println!("Remote peer {:?} subscribed to {:?}", peer_id, topic_hash);
                    alice.publish(msg_a.clone(), topic.clone()).await.unwrap();
                }
                Event::GossipsubMessage(msg) => {
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
        let mut bob = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .bootnodes(vec![bootnode.to_string()])
            .build()
            .unwrap();
        bob.subscribe(&topic);

        loop {
            select! {
                event = bob.next() => {
                    let Some(event) = event else {
                        continue;
                    };

                    match event.unwrap() {
                        Event::GossipsubPeerConnected(peer_id) => {
                            println!("Bob connected to {:?}", peer_id);
                        }
                        Event::GossipsubMessage(msg) => {
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

    let alice_handle = tokio::task::spawn(async move {
        let mut alice = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .build()
            .unwrap();

        let Some(event) = alice.next().await else {
            panic!("expected stream event");
        };

        match event.unwrap() {
            Event::NewListenAddr(addr) => {
                println!("Alice listening on {:?}", addr);
                let maddrs = alice.multiaddrs();
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

                    match event.unwrap() {
                        Event::GossipsubPeerConnected(peer_id) => {
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
        let mut bob = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .bootnodes(vec![bootnode.to_string()])
            .build()
            .unwrap();

        loop {
            select! {
                event = bob.next() => {
                    let Some(event) = event else {
                        continue;
                    };

                    match event.unwrap() {
                        Event::GossipsubPeerConnected(peer_id) => {
                            println!("Bob connected to {:?}", peer_id);
                        }
                        Event::RoutingUpdated(peer_id, addresses) => {
                            println!("Bob's routing table updated by {:?} with addresses {:?}", peer_id, addresses);
                            bob.random_walk().await.unwrap();
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
        let mut charlie = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .bootnodes(vec![bootnode.to_string()])
            .build()
            .unwrap();

        loop {
            let Some(event) = charlie.next().await else {
                break;
            };

            let event = event.unwrap();
            if let Event::GossipsubPeerConnected(peer_id) = event {
                println!("Charlie connected to {:?}", peer_id);
                if charlie.num_peers() == 1 {
                    charlie.random_walk().await.unwrap();
                }

                if charlie.num_peers() == 2 {
                    charlie_tx.send(()).unwrap();
                    return;
                }
            }
        }
    });

    let (res_a, res_b, res_c) = join!(alice_handle, bob_handle, charlie_handle);
    res_a.unwrap();
    res_b.unwrap();
    res_c.unwrap();
}
