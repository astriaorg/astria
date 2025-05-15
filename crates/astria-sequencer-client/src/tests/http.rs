use std::time::Duration;

use astria_core::{
    crypto::SigningKey,
    generated::astria::protocol::{
        asset::v1::AllowedFeeAssetsResponse,
        fees::v1::TransactionFee,
    },
    primitive::v1::Address,
    protocol::transaction::v1::{
        action::Transfer,
        Transaction,
        TransactionBody,
    },
    Protobuf as _,
};
use hex_literal::hex;
use prost::bytes::Bytes;
use serde_json::json;
use tendermint::{
    abci::{
        self,
        Code,
    },
    block::Height,
    merkle,
    tx::Proof,
    Hash,
};
use tendermint_rpc::{
    endpoint::tx,
    response::Wrapper,
    Id,
};
use tokio::time::timeout;
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

const ALICE_ADDRESS_BYTES: [u8; 20] = hex!("1c0c490f1b5528d8173c5de46d131160e4b2c0c3");
const BOB_ADDRESS_BYTES: [u8; 20] = hex!("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a");
const ASTRIA_ADDRESS_PREFIX: &str = "astria";
fn alice_address() -> Address {
    Address::builder()
        .array(ALICE_ADDRESS_BYTES)
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}
fn bob_address() -> Address {
    Address::builder()
        .array(BOB_ADDRESS_BYTES)
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}

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
    raw: impl prost::Message,
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

async fn register_tx_response(server: &MockServer, response: tx::Response) -> MockGuard {
    let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({
        "method": "tx"
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

fn create_signed_transaction() -> Transaction {
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    let alice_key = SigningKey::from(alice_secret_bytes);

    let actions = vec![Transfer {
        to: bob_address(),
        amount: 333_333,
        asset: "nria".parse().unwrap(),
        fee_asset: "nria".parse().unwrap(),
    }
    .into()];
    TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .nonce(1)
        .try_build()
        .unwrap()
        .sign(&alice_key)
}

#[tokio::test]
async fn get_latest_nonce() {
    use astria_core::generated::astria::protocol::accounts::v1::NonceResponse;
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = NonceResponse {
        height: 10,
        nonce: 1,
    };
    let _guard = register_abci_query_response(
        &server,
        &format!("accounts/nonce/{}", alice_address()),
        expected_response,
    )
    .await;

    let actual_response = client
        .get_latest_nonce(alice_address())
        .await
        .unwrap()
        .into_raw();
    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_latest_balance() {
    use astria_core::generated::astria::protocol::accounts::v1::{
        AssetBalance,
        BalanceResponse,
    };

    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = BalanceResponse {
        height: 10,
        balances: vec![AssetBalance {
            denom: "nria".to_string(),
            balance: Some(10u128.pow(18).into()),
        }],
    };
    let _guard = register_abci_query_response(
        &server,
        &format!("accounts/balance/{}", alice_address()),
        expected_response.clone(),
    )
    .await;

    let actual_response = client
        .get_latest_balance(alice_address())
        .await
        .unwrap()
        .into_raw();

    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_allowed_fee_assets() {
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = AllowedFeeAssetsResponse {
        height: 10,
        fee_assets: vec![
            "asset_0".to_string(),
            "asset_1".to_string(),
            "asset_2".to_string(),
        ],
    };

    let _guard = register_abci_query_response(
        &server,
        "asset/allowed_fee_assets",
        expected_response.clone(),
    )
    .await;

    let actual_response = client.get_allowed_fee_assets().await;

    let actual_response = actual_response.unwrap().into_raw();
    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_bridge_account_info() {
    use astria_core::{
        generated::astria::protocol::bridge::v1::BridgeAccountInfoResponse,
        primitive::v1::RollupId,
    };

    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = BridgeAccountInfoResponse {
        height: 10,
        rollup_id: Some(RollupId::from_unhashed_bytes(b"rollup_0").into_raw()),
        asset: Some("asset_0".parse().unwrap()),
        sudo_address: Some(alice_address().into_raw()),
        withdrawer_address: Some(alice_address().into_raw()),
    };

    let _guard =
        register_abci_query_response(&server, "bridge/account_info", expected_response.clone())
            .await;

    let actual_response = client
        .get_bridge_account_info(alice_address())
        .await
        .unwrap()
        .into_raw();

    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_bridge_account_last_transaction_hash() {
    use astria_core::generated::astria::protocol::bridge::v1::BridgeAccountLastTxHashResponse;

    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = BridgeAccountLastTxHashResponse {
        height: 10,
        tx_hash: Some(Bytes::from_static(&[0; 32])),
    };

    let _guard = register_abci_query_response(
        &server,
        "bridge/account_last_tx_hash",
        expected_response.clone(),
    )
    .await;

    let actual_response = client
        .get_bridge_account_last_transaction_hash(alice_address())
        .await
        .unwrap()
        .into_raw();

    assert_eq!(expected_response, actual_response);
}

#[tokio::test]
async fn get_transaction_fee() {
    use astria_core::generated::astria::protocol::fees::v1::TransactionFeeResponse;

    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;

    let expected_response = TransactionFeeResponse {
        height: 10,
        fees: vec![TransactionFee {
            asset: "asset_0".to_string(),
            fee: Some(100.into()),
        }],
    };

    let _guard =
        register_abci_query_response(&server, "transaction/fee", expected_response.clone()).await;

    let actual_response = client
        .get_transaction_fee(create_signed_transaction().into_body())
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
        codespace: String::new(),
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
async fn wait_for_tx_inclusion() {
    let MockSequencer {
        server,
        client,
    } = MockSequencer::start().await;
    let proof = Proof {
        root_hash: Hash::Sha256([0; 32]),
        data: vec![1, 2, 3, 4],
        proof: merkle::Proof {
            total: 1,
            index: 1,
            leaf_hash: Hash::Sha256([0; 32]),
            aunts: vec![],
        },
    };

    let tx_server_response = tx::Response {
        hash: Hash::Sha256([0; 32]),
        height: Height::try_from(1u64).unwrap(),
        index: 1,
        tx_result: abci::types::ExecTxResult {
            code: Code::default(),
            data: Bytes::from(vec![1, 2, 3, 4]),
            log: "ethan was here".to_string(),
            info: String::new(),
            gas_wanted: 0,
            gas_used: 0,
            events: vec![],
            codespace: String::new(),
        },
        tx: vec![],
        proof: Some(proof),
    };

    let _tx_response_guard = register_tx_response(&server, tx_server_response.clone()).await;

    let response = client.wait_for_tx_inclusion(tx_server_response.hash);

    let response = timeout(Duration::from_millis(1000), response)
        .await
        .expect("should have received a transaction response within 1000ms");

    assert_eq!(response.tx_result.code, tx_server_response.tx_result.code);
    assert_eq!(response.tx_result.data, tx_server_response.tx_result.data);
    assert_eq!(response.tx_result.log, tx_server_response.tx_result.log);
    assert_eq!(response.hash, tx_server_response.hash);
}
