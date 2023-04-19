use ed25519_dalek::{Keypair, PublicKey};
use rand::rngs::OsRng;
use std::{collections::HashMap, time::Duration};

use sequencer_relayer::{
    base64_string::Base64String,
    da::CelestiaClient,
    sequencer_block::{get_namespace, IndexedTransaction, SequencerBlock, DEFAULT_NAMESPACE},
};

use crate::helper::{init_environment, init_stack, wait_until_ready};

#[tokio::test]
async fn get_latest_height() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;

    // FIXME: use a more reliable check to ensure that data
    // is available on celestia/the data availability latyer.
    // Right now we have to explicitly wait a sufficient period
    // of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(40)).await;

    let base_url = info.make_bridge_endpoint();
    let client = CelestiaClient::new(base_url).unwrap();

    let height = client.get_latest_height().await.unwrap();
    assert!(height > 0);
}

#[tokio::test]
async fn get_blocks_public_key_filter() {
    // test that get_blocks only gets blocked signed with a specific key
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;

    // FIXME: use a more reliable check to ensure that data
    // is available on celestia/the data availability latyer.
    // Right now we have to explicitly wait a sufficient period
    // of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(40)).await;

    let base_url = info.make_bridge_endpoint();
    let client = CelestiaClient::new(base_url).unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());

    let block_hash = Base64String(vec![99; 32]);
    let block = SequencerBlock {
        block_hash: block_hash.clone(),
        header: Default::default(),
        sequencer_txs: vec![IndexedTransaction {
            index: 0,
            transaction: tx.clone(),
        }],
        rollup_txs: HashMap::new(),
    };

    println!("submitting block");
    let keypair = Keypair::generate(&mut OsRng);
    let submit_block_resp = client.submit_block(block, &keypair).await.unwrap();
    let height = submit_block_resp
        .namespace_to_block_num
        .get(&DEFAULT_NAMESPACE.to_string())
        .unwrap();

    // generate new, different key
    let keypair = Keypair::generate(&mut OsRng);
    let public_key = PublicKey::from_bytes(&keypair.public.to_bytes()).unwrap();
    println!("getting blocks");
    let resp = client.get_blocks(*height, Some(&public_key)).await.unwrap();
    assert!(resp.is_empty());
}

#[tokio::test]
async fn celestia_client() {
    // test submit_block
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;

    // FIXME: use a more reliable check to ensure that data
    // is available on celestia/the data availability latyer.
    // Right now we have to explicitly wait a sufficient period
    // of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(40)).await;

    let base_url = info.make_bridge_endpoint();
    let client = CelestiaClient::new(base_url).unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());
    let secondary_namespace = get_namespace(b"test_namespace");
    let secondary_tx = Base64String(b"noot_was_here_too".to_vec());

    let block_hash = Base64String(vec![99; 32]);
    let mut block = SequencerBlock {
        block_hash: block_hash.clone(),
        header: Default::default(),
        sequencer_txs: vec![IndexedTransaction {
            index: 0,
            transaction: tx.clone(),
        }],
        rollup_txs: HashMap::new(),
    };
    block.rollup_txs.insert(
        secondary_namespace.clone(),
        vec![IndexedTransaction {
            index: 1,
            transaction: secondary_tx.clone(),
        }],
    );

    let keypair = Keypair::generate(&mut OsRng);
    let public_key = PublicKey::from_bytes(&keypair.public.to_bytes()).unwrap();

    let submit_block_resp = client.submit_block(block, &keypair).await.unwrap();
    let height = submit_block_resp
        .namespace_to_block_num
        .get(&DEFAULT_NAMESPACE.to_string())
        .unwrap();

    // test check_block_availability
    let resp = client.check_block_availability(*height).await.unwrap();
    assert_eq!(resp.height, *height);

    // test get_blocks
    let resp = client.get_blocks(*height, Some(&public_key)).await.unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0].block_hash, block_hash);
    assert_eq!(resp[0].header, Default::default());
    assert_eq!(resp[0].sequencer_txs.len(), 1);
    assert_eq!(resp[0].sequencer_txs[0].index, 0);
    assert_eq!(resp[0].sequencer_txs[0].transaction, tx);
    assert_eq!(resp[0].rollup_txs.len(), 1);
    assert_eq!(resp[0].rollup_txs[&secondary_namespace][0].index, 1);
    assert_eq!(
        resp[0].rollup_txs[&secondary_namespace][0].transaction,
        secondary_tx
    );
}
