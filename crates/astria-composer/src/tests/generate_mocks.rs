
use ethers::{providers::{Provider, Middleware}, types::{Transaction, Block}};

use super::test_constants::{MOCK_FILE_DIR, PROVIDER_URL, PENDING_TX_SERIALIZATION_FILE};

pub async fn generate_pending_txs() {
    let pr = Provider::connect(PROVIDER_URL).await.unwrap();

    let archived_block: Block<Transaction> =
        pr.get_block_with_txs(17801777).await.unwrap().unwrap();

    let json_string = serde_json::to_string(&archived_block.transactions).unwrap();
    std::fs::write(PENDING_TX_SERIALIZATION_FILE, json_string).unwrap();
}
