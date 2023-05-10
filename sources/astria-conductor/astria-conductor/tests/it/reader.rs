use std::time::Duration;

use astria_conductor::reader::Reader;
use astria_conductor_test::{
    init_environment,
    init_stack,
    wait_until_ready,
};
use tokio::sync::mpsc;

#[tokio::test]
async fn should_get_new_block() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;
    // FIXME: use a more reliable check to ensure any blocks are
    // available on the sequencer. Right now we have to explicitly
    // wait a sufficient period of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(45)).await;

    let metro_endpoint = info.make_sequencer_api_endpoint();
    let celestia_endpoint = info.make_bridge_endpoint();
    let (executor_tx, _) = mpsc::unbounded_channel();
    let (mut reader, _reader_tx) = Reader::new(
        celestia_endpoint.as_str(),
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
