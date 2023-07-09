use std::collections::HashMap;

use astria_sequencer_relayer::{
    base64_string::Base64String,
    data_availability::CelestiaClient,
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
use ed25519_consensus::{
    SigningKey,
    VerificationKey,
};
use rand_core::OsRng;

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
async fn celestia_client() {
    // test submit_block
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();

    let client = CelestiaClient::builder()
        .endpoint(&bridge_endpoint)
        .bearer_token(&test_env.bearer_token)
        .fee(100_000)
        .build()
        .unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());
    let secondary_namespace = get_namespace(b"test_namespace");
    let secondary_tx = Base64String(b"noot_was_here_too".to_vec());

    let block_hash = Base64String(vec![99; 32]);
    let mut block = SequencerBlock {
        block_hash: block_hash.clone(),
        header: Default::default(),
        last_commit: empty_commit(),
        sequencer_txs: vec![IndexedTransaction {
            block_index: 0,
            transaction: tx.clone(),
        }],
        rollup_txs: HashMap::new(),
    };
    block.rollup_txs.insert(
        secondary_namespace,
        vec![IndexedTransaction {
            block_index: 1,
            transaction: secondary_tx.clone(),
        }],
    );

    let signing_key = SigningKey::new(OsRng);
    let verification_key = VerificationKey::from(&signing_key);

    let submit_block_resp = client
        .submit_block(block, &signing_key, verification_key)
        .await
        .unwrap();
    let _height = submit_block_resp
        .namespace_to_block_num
        .get(&DEFAULT_NAMESPACE)
        .unwrap();
}
