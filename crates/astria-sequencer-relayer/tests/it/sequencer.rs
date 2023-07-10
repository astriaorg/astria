use astria_sequencer_client::Client;
use astria_sequencer_relayer_test::init_test;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_latest_block() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let _client = Client::new(&sequencer_endpoint).unwrap();

    // TODO: fix test env
    //client.get_latest_block().await.unwrap();
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_block() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let _client = Client::new(&sequencer_endpoint).unwrap();

    // TODO: fix test env
    // let resp = client.get_latest_block().await.unwrap();
    // client.get_block(resp.block.header.height).await.unwrap();
}
