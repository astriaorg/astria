use ed25519_consensus::SigningKey;
use sequencer::{
    accounts::{
        query,
        Transfer,
    },
    transaction,
};
use serde_json::json;
use tendermint::Hash;
use tendermint_rpc::{
    endpoint::broadcast::tx_commit::DialectResponse,
    response::Wrapper,
    HttpClient,
    Id,
};
use wiremock::{
    matchers::{
        body_partial_json,
        body_string_contains,
    },
    Mock,
    MockServer,
    ResponseTemplate,
};

use crate::*;

// see astria-sequencer/src/crypto.rs for how these keys/addresses were generated
const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

/// A mock tendermint server for testing.
struct MockTendermintServer {
    mock_server: MockServer,
}

impl MockTendermintServer {
    async fn new() -> Self {
        let mock_server = MockServer::start().await;
        MockTendermintServer {
            mock_server,
        }
    }

    fn address(&self) -> String {
        format!("http://{}", self.mock_server.address())
    }

    async fn register_abci_query_response(&self, query_path: &str, response: &query::Response) {
        use borsh::BorshSerialize as _;
        let expected_body = json!({
            "method": "abci_query"
        });
        let response = tendermint_rpc::endpoint::abci_query::Response {
            response: tendermint_rpc::endpoint::abci_query::AbciQuery {
                value: response.try_to_vec().unwrap(),
                ..Default::default()
            },
        };
        let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
        Mock::given(body_partial_json(&expected_body))
            .and(body_string_contains(query_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&wrapper)
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&self.mock_server)
            .await;
    }

    async fn register_broadcast_tx_sync_response(&self, response: tx_sync::Response) {
        let expected_body = json!({
            "method": "broadcast_tx_sync"
        });
        let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
        Mock::given(body_partial_json(&expected_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&wrapper)
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&self.mock_server)
            .await;
    }

    async fn register_broadcast_tx_commit_response(
        &self,
        response: DialectResponse<tendermint_rpc::dialect::v0_37::Event>,
    ) {
        let expected_body = json!({
            "method": "broadcast_tx_commit"
        });
        let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
        Mock::given(body_partial_json(&expected_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&wrapper)
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&self.mock_server)
            .await;
    }
}

fn create_signed_transaction() -> transaction::Signed {
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    let alice_keypair = SigningKey::from(alice_secret_bytes);

    let actions = vec![transaction::Action::TransferAction(Transfer::new(
        Address::try_from_str(BOB_ADDRESS).unwrap(),
        Balance::from(333_333),
    ))];
    let tx = transaction::Unsigned::new_with_actions(Nonce::from(1), actions);
    tx.into_signed(&alice_keypair)
}

#[tokio::test]
async fn get_nonce_and_balance() {
    let server = MockTendermintServer::new().await;

    let nonce_response = Nonce::from(0);
    server
        .register_abci_query_response(
            "accounts/nonce",
            &query::Response::NonceResponse(nonce_response),
        )
        .await;

    let balance_response = Balance::from(10_u128.pow(18));
    server
        .register_abci_query_response(
            "accounts/balance",
            &query::Response::BalanceResponse(balance_response),
        )
        .await;

    let client = HttpClient::new(server.address().as_str()).unwrap();
    let address = Address::try_from_str(ALICE_ADDRESS).unwrap();
    let nonce = client.get_nonce(&address, None).await.unwrap();
    assert_eq!(nonce, nonce_response);
    let balance = client.get_balance(&address, None).await.unwrap();
    assert_eq!(balance, balance_response);
}

#[tokio::test]
async fn submit_tx_sync() {
    let server = MockTendermintServer::new().await;

    let server_response = tx_sync::Response {
        code: 0.into(),
        data: vec![].into(),
        log: String::new(),
        hash: Hash::Sha256([0; 32]),
    };
    server
        .register_broadcast_tx_sync_response(server_response.clone())
        .await;
    let signed_tx = create_signed_transaction();

    let client = HttpClient::new(server.address().as_str()).unwrap();
    let response = client.submit_transaction_sync(signed_tx).await.unwrap();
    assert_eq!(response.code, server_response.code);
    assert_eq!(response.data, server_response.data);
    assert_eq!(response.log, server_response.log);
    assert_eq!(response.hash, server_response.hash);
}

#[tokio::test]
async fn submit_tx_commit() {
    use tendermint_rpc::dialect;

    let server = MockTendermintServer::new().await;

    let server_response = DialectResponse::<tendermint_rpc::dialect::v0_37::Event> {
        check_tx: dialect::CheckTx::default(),
        deliver_tx: dialect::DeliverTx::default(),
        hash: Hash::Sha256([0; 32]),
        height: Height::from(1u32),
    };
    server
        .register_broadcast_tx_commit_response(server_response)
        .await;

    let signed_tx = create_signed_transaction();

    let client = HttpClient::new(server.address().as_str()).unwrap();
    let response = client.submit_transaction_commit(signed_tx).await.unwrap();
    assert_eq!(response.check_tx.code, 0.into());
    assert_eq!(response.deliver_tx.code, 0.into());
}
