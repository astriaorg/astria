use astria_sequencer_relayer::{
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
};
use astria_sequencer_relayer_test::init_test;
use tendermint::Block;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn header_verify_hashes() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let resp = client.get_latest_block().await.unwrap();
    let block = Block::try_from(resp.block).unwrap();
    let sequencer_block = SequencerBlock::from_cosmos_block(block).unwrap();
    sequencer_block.verify_data_hash().unwrap();
    sequencer_block.verify_block_hash().unwrap();
}
