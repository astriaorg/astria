use crate::sequencer::SequencerClient;
use sequencer_relayer_test::init_test;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn test_header_to_tendermint_header() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let resp = client.get_latest_block().await.unwrap();
    let tm_header = &resp.block.header.to_tendermint_header().unwrap();
    let tm_header_hash = tm_header.hash();
    assert_eq!(tm_header_hash.as_bytes(), &resp.block_id.hash.0);
}
