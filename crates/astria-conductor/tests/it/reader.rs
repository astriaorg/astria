use std::{
    sync::Arc,
    time::Duration,
};

use astria_conductor::{
    block_verifier::BlockVerifier,
    reader::Reader,
};
use tokio::sync::mpsc;

use crate::helper::init_test;

#[ignore = "requires heavy kubernetes test environment"]
#[tokio::test]
async fn should_get_new_block() {
    let test_env = init_test().await;

    let metro_endpoint = test_env.sequencer_endpoint();
    let bridge_endpoint = test_env.bridge_endpoint();

    let block_validator = BlockVerifier::new(metro_endpoint.as_str()).unwrap();
    let (executor_tx, _) = mpsc::unbounded_channel();
    let (mut reader, _reader_tx) = Reader::new(
        bridge_endpoint.as_str(),
        executor_tx,
        Arc::new(block_validator),
    )
    .await
    .unwrap();

    // FIXME - this is NOT a good test, but it gets us to a passing state.
    let mut blocks = vec![];
    for _ in 0..30 {
        blocks = reader.get_new_blocks().await.unwrap();
        if !blocks.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    assert!(!blocks.is_empty());
}
