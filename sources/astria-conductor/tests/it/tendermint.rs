use crate::helper::{init_environment, init_stack, wait_until_ready};
use astria_conductor::tendermint::TendermintClient;
use std::time::Duration;

#[tokio::test]
async fn should_get_validator_set() {
    let podman = init_environment();
    let info = init_stack(&podman).await;
    wait_until_ready(&podman, &info.pod_name).await;
    // FIXME: use a more reliable check to ensure any blocks are
    // available on the sequencer. Right now we have to explicitly
    // wait a sufficient period of time. This is flaky.
    tokio::time::sleep(Duration::from_secs(30)).await;

    let sequencer_endpoint = info.make_sequencer_api_endpoint();
    let client = TendermintClient::new(sequencer_endpoint).unwrap();
    let resp = client.get_validator_set(1).await.unwrap();
    println!("ValidatorSet: {:?}", resp);
}
