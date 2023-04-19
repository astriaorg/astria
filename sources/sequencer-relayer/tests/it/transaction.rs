use std::time::Duration;

use sequencer_relayer::{sequencer::SequencerClient, transaction};

use crate::helper::{init_environment, init_stack, wait_until_ready};

#[tokio::test]
async fn txs_to_data_hash() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;
    let cosmos_endpoint = info.make_sequencer_endpoint();

    // FIXME: use a more reliable check to ensure any blocks are
    // available on the sequencer. Right now we have to explicitly
    // wait a sufficient period of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(20)).await;

    let client = SequencerClient::new(cosmos_endpoint).unwrap();
    let resp = client.get_latest_block().await.unwrap();
    let data_hash = transaction::txs_to_data_hash(&resp.block.data.txs);
    assert_eq!(
        data_hash.as_bytes(),
        &resp.block.header.data_hash.unwrap().0
    );
}
