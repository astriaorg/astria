use astria_conductor::tendermint::TendermintClient;

use crate::helper::init_test;

#[ignore = "requires heavy kubernetes test environment"]
#[tokio::test]
async fn should_get_validator_set() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = TendermintClient::new(sequencer_endpoint).unwrap();
    let _resp = client.get_validator_set(1).await.unwrap();
}
