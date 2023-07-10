use astria_sequencer_client::Client;
// use astria_sequencer_relayer::types::ParsedSequencerBlockData;
use astria_sequencer_relayer_test::init_test;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn header_verify_hashes() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let _client = Client::new(&sequencer_endpoint).unwrap();

    // TODO: fix test env
    // let resp = client.get_latest_block().await.unwrap();
    // let sequencer_block = ParsedSequencerBlockData::from_tendermint_block(resp.block).unwrap();
    // sequencer_block.verify_data_hash().unwrap();
    // sequencer_block.verify_block_hash().unwrap();
}
