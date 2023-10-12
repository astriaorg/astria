use ed25519_consensus::SigningKey;
use hex_literal::hex;
use proto::native::sequencer::v1alpha1::{
    Address,
    SignedTransaction,
    TransferAction,
    UnsignedTransaction,
};
use serde_json::json;
use tendermint::{
    block::Height,
    Hash,
};
use tendermint_rpc::{
    endpoint::broadcast::tx_commit::v0_37::DialectResponse,
    response::Wrapper,
    Id,
};
use wiremock::{
    matchers::{
        body_partial_json,
        body_string_contains,
    },
    Mock,
    MockGuard,
    MockServer,
    ResponseTemplate,
};

use crate::{
    tendermint_rpc::endpoint::broadcast::tx_sync,
    HttpClient,
    SequencerClientExt as _,
};

const ALICE_ADDRESS: [u8; 20] = hex!("1c0c490f1b5528d8173c5de46d131160e4b2c0c3");
const BOB_ADDRESS: Address = Address(hex!("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a"));

struct MockSequencer {
    server: MockServer,
    client: HttpClient,
}

impl MockSequencer {
    async fn start() -> Self {
        let server = MockServer::start().await;
        let client = HttpClient::new(&*format!("http://{}", server.address())).unwrap();
        Self {
            server,
            client,
        }
    }
}

async fn register_abci_query_response(
    server: &MockServer,
    query_path: &str,
    raw: impl proto::Message,
) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: raw.encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({
        "method": "abci_query"
    })))
    .and(body_string_contains(query_path))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

async fn register_broadcast_tx_sync_response(
    server: &MockServer,
    response: tx_sync::Response,
) -> MockGuard {
    let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({
        "method": "broadcast_tx_sync"
    })))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

async fn register_broadcast_tx_commit_response(
    server: &MockServer,
    response: DialectResponse,
) -> MockGuard {
    let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({
        "method": "broadcast_tx_commit"
    })))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

fn create_signed_transaction() -> SignedTransaction {
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    let alice_key = SigningKey::from(alice_secret_bytes);

    let actions = vec![
        TransferAction {
            to: BOB_ADDRESS,
            amount: 333_333,
        }
        .into(),
    ];
    UnsignedTransaction {
        nonce: 1,
        actions,
    }
    .into_signed(&alice_key)
}

#[tokio::test]
async fn get_latest_nonce() {
    use proto::generated::sequencer::v1alpha1::NonceResponse;
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = NonceResponse {
        height: 10,
        nonce: 1,
    };
    let _guard =
        register_abci_query_response(&server, "accounts/nonce/", expected_response.clone()).await;

    let actual_response = client
        .get_latest_nonce(ALICE_ADDRESS)
        .await
        .unwrap()
        .into_raw();
    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_latest_balance() {
    use proto::generated::sequencer::v1alpha1::BalanceResponse;
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = BalanceResponse {
        height: 10,
        balance: Some(10u128.pow(18).into()),
    };
    let _guard =
        register_abci_query_response(&server, "accounts/balance/", expected_response.clone()).await;

    let actual_response = client
        .get_latest_balance(ALICE_ADDRESS)
        .await
        .unwrap()
        .into_raw();
    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn submit_tx_sync() {
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let server_response = tx_sync::Response {
        code: 0.into(),
        data: vec![].into(),
        log: String::new(),
        hash: Hash::Sha256([0; 32]),
    };
    let _guard = register_broadcast_tx_sync_response(&server, server_response.clone()).await;
    let signed_tx = create_signed_transaction();

    let response = client.submit_transaction_sync(signed_tx).await.unwrap();
    assert_eq!(response.code, server_response.code);
    assert_eq!(response.data, server_response.data);
    assert_eq!(response.log, server_response.log);
    assert_eq!(response.hash, server_response.hash);
}

#[tokio::test]
async fn submit_tx_commit() {
    use tendermint_rpc::dialect;

    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let server_response = DialectResponse {
        check_tx: dialect::CheckTx::default(),
        deliver_tx: dialect::DeliverTx::default(),
        hash: Hash::Sha256([0; 32]),
        height: Height::from(1u32),
    };
    let _guard = register_broadcast_tx_commit_response(&server, server_response).await;

    let signed_tx = create_signed_transaction();

    let response = client.submit_transaction_commit(signed_tx).await.unwrap();
    assert_eq!(response.check_tx.code, 0.into());
    assert_eq!(response.tx_result.code, 0.into());
}
