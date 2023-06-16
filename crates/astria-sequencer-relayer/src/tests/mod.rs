use astria_sequencer_relayer_test::init_test;
use tendermint::{
    Block,
    Hash,
};
use tendermint_proto::Protobuf;

use crate::sequencer::SequencerClient;

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn test_header_to_tendermint_header() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let resp = client.get_latest_block().await.unwrap();
    let block_id_hash = Hash::decode_vec(&resp.block_id.hash.0).unwrap();
    let block = Block::try_from(resp.block).unwrap();
    let tm_header_hash = block.header.hash();
    assert_eq!(tm_header_hash, block_id_hash);
}
