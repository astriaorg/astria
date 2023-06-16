use std::collections::HashMap;

use astria_sequencer_relayer::{
    base64_string::Base64String,
    data_availability::CelestiaClientBuilder,
    sequencer_block::{
        get_namespace,
        IndexedTransaction,
        SequencerBlock,
        DEFAULT_NAMESPACE,
    },
    types::{
        BlockId,
        Commit,
        Parts,
    },
};
use astria_sequencer_relayer_test::init_test;
use ed25519_dalek::{
    Keypair,
    PublicKey,
};
use rand::rngs::OsRng;
use tendermint::{
    account,
    block::{
        header::Version,
        Header,
        Height,
    },
    chain,
    hash,
    AppHash,
    Hash,
    Time,
};

fn make_header() -> Header {
    Header {
        version: Version {
            block: 0,
            app: 0,
        },
        chain_id: {
            match chain::Id::try_from("chain") {
                Ok(id) => id,
                _ => panic!("chain id construction failed"),
            }
        },
        height: Height::from(0_u32),
        time: Time::now(),
        last_block_id: None,
        last_commit_hash: None,
        data_hash: None,
        validators_hash: Hash::default(),
        next_validators_hash: Hash::default(),
        consensus_hash: Hash::default(),
        app_hash: AppHash::default(),
        last_results_hash: None,
        evidence_hash: None,
        proposer_address: account::Id::new([0; 20]),
    }
}

fn empty_commit() -> Commit {
    Commit {
        height: "0".to_string(),
        round: 0,
        block_id: BlockId {
            hash: Base64String(vec![]),
            part_set_header: Parts {
                total: 0,
                hash: Base64String(vec![]),
            },
        },
        signatures: vec![],
    }
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_latest_height() {
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();
    let height = client.get_latest_height().await.unwrap();
    assert!(height > 0);
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_blocks_public_key_filter() {
    // test that get_blocks only gets blocked signed with a specific key

    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());

    let block_hash = Hash::from_bytes(hash::Algorithm::Sha256, &[99; 32]).unwrap();
    let block = SequencerBlock {
        block_hash,
        header: make_header(),
        last_commit: empty_commit(),
        sequencer_transactions: vec![IndexedTransaction {
            block_index: 0,
            transaction: tx.0.clone(),
        }],
        rollup_transactions: HashMap::new(),
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
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());
    let secondary_namespace = get_namespace(b"test_namespace");
    let secondary_tx = Base64String(b"noot_was_here_too".to_vec());

    let block_hash = Hash::from_bytes(hash::Algorithm::Sha256, &[99; 32]).unwrap();
    let mut block = SequencerBlock {
        block_hash,
        header: make_header(),
        last_commit: empty_commit(),
        sequencer_transactions: vec![IndexedTransaction {
            block_index: 0,
            transaction: tx.0.clone(),
        }],
        rollup_transactions: HashMap::new(),
    };
    block.rollup_transactions.insert(
        secondary_namespace.clone(),
        vec![IndexedTransaction {
            block_index: 1,
            transaction: secondary_tx.0.clone(),
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
    assert_eq!(resp[0].header, make_header());
    assert_eq!(resp[0].sequencer_transactions.len(), 1);
    assert_eq!(resp[0].sequencer_transactions[0].block_index, 0);
    assert_eq!(resp[0].sequencer_transactions[0].transaction, tx.0);
    assert_eq!(resp[0].rollup_transactions.len(), 1);
    assert_eq!(
        resp[0].rollup_transactions[&secondary_namespace][0].block_index,
        1
    );
    assert_eq!(
        resp[0].rollup_transactions[&secondary_namespace][0].transaction,
        secondary_tx.0
    );
}
