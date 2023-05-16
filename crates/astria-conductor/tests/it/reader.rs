use std::time::Duration;

use astria_conductor::reader::Reader;
use astria_conductor_test::init_test;
use tokio::sync::mpsc;

#[tokio::test]
async fn should_get_new_block() {
    let test_env = init_test().await;

    let metro_endpoint = test_env.sequencer_endpoint();
    let bridge_endpoint = test_env.bridge_endpoint();

    let (executor_tx, _) = mpsc::unbounded_channel();
    let (mut reader, _reader_tx) = Reader::new(
        bridge_endpoint.as_str(),
        metro_endpoint.as_str(),
        executor_tx,
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
