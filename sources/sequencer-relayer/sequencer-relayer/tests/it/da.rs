use ed25519_dalek::{Keypair, PublicKey};
use rand::rngs::OsRng;
use std::collections::HashMap;

use sequencer_relayer::{
    base64_string::Base64String,
    da::CelestiaClient,
    sequencer_block::{get_namespace, IndexedTransaction, SequencerBlock, DEFAULT_NAMESPACE},
};

use sequencer_relayer_test::init_test;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_latest_height() {
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClient::new(bridge_endpoint).unwrap();
    let height = client.get_latest_height().await.unwrap();
    assert!(height > 0);
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_blocks_public_key_filter() {
    // test that get_blocks only gets blocked signed with a specific key

    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClient::new(bridge_endpoint).unwrap();

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
#[ignore = "very slow init of test environment"]
async fn celestia_client() {
    // test submit_block
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClient::new(bridge_endpoint).unwrap();

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
