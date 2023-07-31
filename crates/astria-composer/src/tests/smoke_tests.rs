use std::time::Duration;

use astria_sequencer_client::client::test_utils::MockTendermintServer;

use astria_sequencer::accounts::types::Address as SequencerAddress;
use astria_sequencer_client::Client as SequencerClient;
use ethers::providers::{Middleware, Provider};
use ethers::types::{Block, Transaction};
use tendermint_rpc::endpoint::broadcast::tx_sync::{self, Response};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::searcher::Searcher;
use crate::tests::{test_constants, ComposerMockProvider};

pub async fn setup_test_provider() -> (Provider<ComposerMockProvider>, ComposerMockProvider, u64) {
    let mut file_path = std::env::current_dir().unwrap();
    file_path.push("src/tests");
    file_path.push(test_constants::MOCK_FILE_DIR);
    file_path.push(test_constants::PENDING_TX_SERIALIZATION_FILE);

    let file = std::fs::File::open(file_path).unwrap();
    let reader = std::io::BufReader::new(file);

    let pending_tx_vec: Vec<Transaction> = serde_json::from_reader(reader).unwrap();

    let (pr, mut mock) = ComposerMockProvider::init_provider();

    // Send all the mock data to the mock provider
    for tx in &pending_tx_vec {
        mock.push(tx.clone().hash).unwrap();
    }

    mock.setup_subscription().await;

    (pr, mock, pending_tx_vec.len() as u64)
}

pub async fn setup_mock_sequencer_client(num_tx: u64) -> (SequencerClient, Response) {
    let mock_tendermint_server = MockTendermintServer::new().await;
    let server_res = mock_tendermint_server.register_dummy_sync_res(Some(num_tx)).await;

    let sequender_client = SequencerClient::new(mock_tendermint_server.address().as_str()).unwrap();

    (sequender_client, server_res)
}

#[tokio::test]
pub async fn smoke_test_searcher() {
    let (pr, mock, num_tx) = setup_test_provider().await;

    let (sq_client, seq_expected_res) = setup_mock_sequencer_client(num_tx).await;

    let searcher = Searcher::<ComposerMockProvider>::build(pr, sq_client, "740".to_string());

    // Test should complete within 5 seconds, since block frequency is supposed to be 1 sec
    let result = timeout(Duration::from_secs(5), async {
        searcher.run().await.unwrap();
    }).await;

    assert!(result.is_ok());
}
