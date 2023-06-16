use astria_sequencer_relayer::{
    sequencer::SequencerClient,
    transaction,
};
use astria_sequencer_relayer_test::init_test;
use tendermint::{
    hash,
    Hash,
};

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn txs_to_data_hash() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let resp = client.get_latest_block().await.unwrap();
    let data_hash = transaction::txs_to_data_hash(&resp.block.data.txs);
    assert_eq!(
        data_hash,
        Hash::from_bytes(
            hash::Algorithm::Sha256,
            &resp.block.header.data_hash.unwrap().0
        )
        .unwrap()
    );
}
