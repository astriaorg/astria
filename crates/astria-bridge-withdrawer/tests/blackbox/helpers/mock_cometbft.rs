use std::time::Duration;

use astria_core::{
    generated::protocol::account::v1alpha1::NonceResponse,
    primitive::v1::asset,
    protocol::{
        account::v1alpha1::AssetBalance,
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
    },
};
use prost::Message as _;
use sequencer_client::SignedTransaction;
use tendermint::{
    abci::{
        response::CheckTx,
        types::ExecTxResult,
    },
    block::Height,
    chain,
};
use tendermint_rpc::{
    endpoint::{
        broadcast::{
            tx_commit,
            tx_sync,
        },
        tx,
    },
    response,
};
use tracing::debug;
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

use super::test_bridge_withdrawer::default_native_asset;

const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";

async fn _register_default_chain_id_guard(cometbft_mock: &MockServer) -> MockGuard {
    _register_genesis_chain_id_response(SEQUENCER_CHAIN_ID, cometbft_mock).await
}

async fn _register_default_fee_asset_ids_guard(cometbft_mock: &MockServer) -> MockGuard {
    let fee_assets = vec![default_native_asset()];
    _register_allowed_fee_assets_response(fee_assets, cometbft_mock).await
}

async fn _register_default_min_expected_fee_asset_balance_guard(
    cometbft_mock: &MockServer,
) -> MockGuard {
    _register_get_latest_balance(
        vec![AssetBalance {
            denom: default_native_asset(),
            balance: 1_000_000u128,
        }],
        cometbft_mock,
    )
    .await
}

fn make_tx_commit_success_response() -> tx_commit::Response {
    tx_commit::Response {
        check_tx: CheckTx::default(),
        tx_result: ExecTxResult::default(),
        hash: vec![0u8; 32].try_into().unwrap(),
        height: Height::default(),
    }
}

fn make_tx_commit_check_tx_failure_response() -> tx_commit::Response {
    tx_commit::Response {
        check_tx: CheckTx {
            code: 1.into(),
            ..CheckTx::default()
        },
        tx_result: ExecTxResult::default(),
        hash: vec![0u8; 32].try_into().unwrap(),
        height: Height::default(),
    }
}

fn make_tx_commit_deliver_tx_failure_response() -> tx_commit::Response {
    tx_commit::Response {
        check_tx: CheckTx::default(),
        tx_result: ExecTxResult {
            code: 1.into(),
            ..ExecTxResult::default()
        },
        hash: vec![0u8; 32].try_into().unwrap(),
        height: Height::default(),
    }
}

async fn _register_genesis_chain_id_response(chain_id: &str, server: &MockServer) -> MockGuard {
    use tendermint::{
        consensus::{
            params::{
                AbciParams,
                ValidatorParams,
            },
            Params,
        },
        genesis::Genesis,
        time::Time,
    };
    let response = tendermint_rpc::endpoint::genesis::Response::<serde_json::Value> {
        genesis: Genesis {
            genesis_time: Time::from_unix_timestamp(1, 1).unwrap(),
            chain_id: chain::Id::try_from(chain_id).unwrap(),
            initial_height: 1,
            consensus_params: Params {
                block: tendermint::block::Size {
                    max_bytes: 1024,
                    max_gas: 1024,
                    time_iota_ms: 1000,
                },
                evidence: tendermint::evidence::Params {
                    max_age_num_blocks: 1000,
                    max_age_duration: tendermint::evidence::Duration(Duration::from_secs(3600)),
                    max_bytes: 1_048_576,
                },
                validator: ValidatorParams {
                    pub_key_types: vec![tendermint::public_key::Algorithm::Ed25519],
                },
                version: None,
                abci: AbciParams::default(),
            },
            validators: vec![],
            app_hash: tendermint::hash::AppHash::default(),
            app_state: serde_json::Value::Null,
        },
    };

    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(serde_json::json!({"method": "genesis"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&wrapper)
                .append_header("Content-Type", "application/json"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount_as_scoped(server)
        .await
}

async fn _register_allowed_fee_assets_response(
    fee_assets: Vec<asset::Denom>,
    cometbft_mock: &MockServer,
) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: astria_core::protocol::asset::v1alpha1::AllowedFeeAssetsResponse {
                fee_assets,
                height: 1,
            }
            .into_raw()
            .encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(
        serde_json::json!({"method": "abci_query"}),
    ))
    .and(body_string_contains("asset/allowed_fee_asset_ids"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(cometbft_mock)
    .await
}

async fn _register_get_latest_balance(
    balances: Vec<AssetBalance>,
    server: &MockServer,
) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: astria_core::protocol::account::v1alpha1::BalanceResponse {
                balances,
                height: 1,
            }
            .into_raw()
            .encode_to_vec(),
            ..Default::default()
        },
    };

    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(
        serde_json::json!({"method": "abci_query"}),
    ))
    .and(body_string_contains("accounts/balance"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

async fn _register_last_bridge_tx_hash_guard(
    server: &MockServer,
    response: BridgeAccountLastTxHashResponse,
) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: response.into_raw().encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(
        serde_json::json!({"method": "abci_query"}),
    ))
    .and(body_string_contains("bridge/account_last_tx_hash"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

async fn register_get_nonce_response(server: &MockServer, response: NonceResponse) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: response.encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(
        serde_json::json!({"method": "abci_query"}),
    ))
    .and(body_string_contains("accounts/nonce"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(&wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
    .mount_as_scoped(server)
    .await
}

async fn _register_tx_guard(server: &MockServer, response: tx::Response) -> MockGuard {
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(serde_json::json!({"method": "tx"})))
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
    response: tx_commit::Response,
) -> MockGuard {
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(serde_json::json!({
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

/// Convert a `Request` object to a `SignedTransaction`
fn signed_tx_from_request(request: &wiremock::Request) -> SignedTransaction {
    use astria_core::generated::protocol::transaction::v1alpha1::SignedTransaction as RawSignedTransaction;
    use prost::Message as _;

    let wrapped_tx_sync_req: tendermint_rpc::request::Wrapper<tx_sync::Request> =
        serde_json::from_slice(&request.body)
            .expect("deserialize to JSONRPC wrapped tx_sync::Request");
    let raw_signed_tx = RawSignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
        .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
        .expect("can't convert raw signed tx to checked signed tx");
    debug!(?signed_tx, "sequencer mock received signed transaction");

    signed_tx
}
