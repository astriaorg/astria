use std::time::Duration;

use crate::helper::{init_environment, init_stack, wait_until_ready};
use sequencer_relayer::sequencer::SequencerClient;

#[tokio::test]
async fn get_latest_block() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;
    let cosmos_endpoint = info.make_sequencer_endpoint();

    // FIXME: use a more reliable check to ensure any blocks are
    // available on the sequencer. Right now we have to explicitly
    // wait a sufficient period of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(30)).await;

    let client = SequencerClient::new(cosmos_endpoint).unwrap();
    client.get_latest_block().await.unwrap();
}

#[tokio::test]
async fn get_block() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;
    let cosmos_endpoint = info.make_sequencer_endpoint();

    // FIXME: use a more reliable check to ensure any blocks are
    // available on the sequencer. Right now we have to explicitly
    // wait a sufficient period of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(30)).await;

    let client = SequencerClient::new(cosmos_endpoint).unwrap();
    let resp = client.get_latest_block().await.unwrap();
    let height: u64 = resp.block.header.height.parse().unwrap();
    client.get_block(height).await.unwrap();
}
