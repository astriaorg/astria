use crate::helper::init_test;
use sequencer_relayer::sequencer::SequencerClient;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_latest_block() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    client.get_latest_block().await.unwrap();
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_block() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let resp = client.get_latest_block().await.unwrap();
    let height: u64 = resp.block.header.height.parse().unwrap();
    client.get_block(height).await.unwrap();
}
