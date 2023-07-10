use astria_sequencer_client::Client;
// use astria_sequencer_relayer::transaction;
use astria_sequencer_relayer_test::init_test;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn txs_to_data_hash() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let _client = Client::new(&sequencer_endpoint).unwrap();

    // TODO: fix test env
    // let resp = client.get_latest_block().await.unwrap();
    // let data_hash = transaction::txs_to_data_hash(&resp.block.data);
    // assert_eq!(
    //     data_hash.as_bytes(),
    //     resp.block.header.data_hash.unwrap().as_bytes()
    // );
}
