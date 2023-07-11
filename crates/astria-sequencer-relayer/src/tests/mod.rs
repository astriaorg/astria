use std::time::Duration;

use astria_sequencer_relayer_test::init_test;
use backon::{
    ConstantBuilder,
    Retryable as _,
};
use eyre::WrapErr as _;

use crate::{
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
    transaction,
    types::BlockResponse,
};

async fn get_latest_block(client: SequencerClient) -> BlockResponse {
    async fn call(client: SequencerClient) -> eyre::Result<BlockResponse> {
        client
            .get_latest_block()
            .await
            .wrap_err("failed to get latest block")
    }
    let backoff = ConstantBuilder::default()
        .with_delay(Duration::from_secs(5))
        .with_max_times(3);
    (move || call(client.clone()))
        .retry(&backoff)
        .await
        .expect("failed to get tendermint header after 3 retries")
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn header_to_tendermint_header_via_metro() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();

    let rsp = get_latest_block(client).await;
    let header = rsp
        .block
        .header
        .to_tendermint_header()
        .expect("failed to extract tendermint header from block");
    let header_hash = header.hash();
    assert_eq!(header_hash.as_bytes(), &rsp.block_id.hash.0);
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_block_after_latest_block() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();
    let rsp = get_latest_block(client.clone()).await;
    let height: u64 = rsp
        .block
        .header
        .height
        .parse()
        .expect("failed to parse height from block header response");
    let _ = client
        .get_block(height)
        .await
        .expect("failed to get block from height extracted from latest block");
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn header_verify_hashes() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();
    let rsp = get_latest_block(client).await;
    let sequencer_block = SequencerBlock::from_cosmos_block(rsp.block).unwrap();
    sequencer_block.verify_data_hash().unwrap();
    sequencer_block.verify_block_hash().unwrap();
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn txs_to_data_hash() {
    let test_env = init_test().await;
    let sequencer_endpoint = test_env.sequencer_endpoint();
    let client = SequencerClient::new(sequencer_endpoint).unwrap();
    let rsp = get_latest_block(client).await;
    let data_hash = transaction::txs_to_data_hash(&rsp.block.data.txs);
    assert_eq!(data_hash.as_bytes(), &rsp.block.header.data_hash.unwrap().0);
}
