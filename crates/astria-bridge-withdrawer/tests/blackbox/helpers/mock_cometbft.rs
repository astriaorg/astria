use std::time::Duration;

use astria_core::{
    primitive::v1::asset,
    protocol::bridge::v1alpha1::BridgeAccountLastTxHashResponse,
};
use prost::Message as _;
use sequencer_client::{
    NonceResponse,
    SignedTransaction,
};
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

use super::test_bridge_withdrawer::{
    default_native_asset,
    DEFAULT_IBC_DENOM,
    SEQUENCER_CHAIN_ID,
};

#[must_use]
pub fn make_tx_commit_success_response() -> tx_commit::Response {
    tx_commit::Response {
        check_tx: CheckTx::default(),
        tx_result: ExecTxResult::default(),
        hash: vec![0u8; 32].try_into().unwrap(),
        height: Height::default(),
    }
}

#[must_use]
pub fn make_tx_commit_check_tx_failure_response() -> tx_commit::Response {
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

#[must_use]
pub fn make_tx_commit_deliver_tx_failure_response() -> tx_commit::Response {
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

pub async fn mount_default_chain_id(cometbft_mock: &MockServer) {
    mount_genesis_chain_id_response(SEQUENCER_CHAIN_ID, cometbft_mock).await;
}

pub async fn mount_default_chain_id_guard_as_scoped(cometbft_mock: &MockServer) -> MockGuard {
    mount_genesis_chain_id_response_as_scoped(SEQUENCER_CHAIN_ID, cometbft_mock).await
}

pub async fn mount_native_fee_asset(cometbft_mock: &MockServer) {
    let fee_assets = vec![default_native_asset()];
    mount_allowed_fee_assets_response(fee_assets, cometbft_mock).await;
}

pub async fn mount_native_fee_asset_as_scoped(cometbft_mock: &MockServer) -> MockGuard {
    let fee_assets = vec![DEFAULT_IBC_DENOM.parse().unwrap()];
    mount_allowed_fee_assets_response_as_scoped(fee_assets, cometbft_mock).await
}

pub async fn mount_ibc_fee_asset(cometbft_mock: &MockServer) {
    let fee_assets = vec![DEFAULT_IBC_DENOM.parse().unwrap()];
    mount_allowed_fee_assets_response(fee_assets, cometbft_mock).await;
}

pub async fn mount_ibc_fee_asset_as_scoped(cometbft_mock: &MockServer) -> MockGuard {
    let fee_assets = vec![default_native_asset()];
    mount_allowed_fee_assets_response_as_scoped(fee_assets, cometbft_mock).await
}

pub async fn mount_genesis_chain_id_response(chain_id: &str, server: &MockServer) {
    prepare_genesis_chain_id_response(chain_id)
        .mount(server)
        .await;
}

pub async fn mount_genesis_chain_id_response_as_scoped(
    chain_id: &str,
    server: &MockServer,
) -> MockGuard {
    prepare_genesis_chain_id_response(chain_id)
        .mount_as_scoped(server)
        .await
}

fn prepare_genesis_chain_id_response(chain_id: &str) -> Mock {
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
                .set_body_json(wrapper)
                .append_header("Content-Type", "application/json"),
        )
        .up_to_n_times(1)
        .expect(1)
}

pub async fn mount_allowed_fee_assets_response(
    fee_assets: Vec<asset::Denom>,
    cometbft_mock: &MockServer,
) {
    prepare_allowed_fee_assets_response(fee_assets)
        .mount(cometbft_mock)
        .await;
}

pub async fn mount_allowed_fee_assets_response_as_scoped(
    fee_assets: Vec<asset::Denom>,
    cometbft_mock: &MockServer,
) -> MockGuard {
    prepare_allowed_fee_assets_response(fee_assets)
        .mount_as_scoped(cometbft_mock)
        .await
}

fn prepare_allowed_fee_assets_response(fee_assets: Vec<asset::Denom>) -> Mock {
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
    .and(body_string_contains("asset/allowed_fee_assets"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
}

pub async fn mount_last_bridge_tx_hash_response(
    server: &MockServer,
    response: BridgeAccountLastTxHashResponse,
) {
    prepare_last_bridge_tx_hash_response(response)
        .mount(server)
        .await;
}

pub async fn mount_last_bridge_tx_hash_response_as_scoped(
    server: &MockServer,
    response: BridgeAccountLastTxHashResponse,
) -> MockGuard {
    prepare_last_bridge_tx_hash_response(response)
        .mount_as_scoped(server)
        .await
}

fn prepare_last_bridge_tx_hash_response(response: BridgeAccountLastTxHashResponse) -> Mock {
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
            .set_body_json(wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
}

pub async fn mount_get_nonce_response(server: &MockServer, response: NonceResponse) {
    prepare_get_nonce_response(response).mount(server).await;
}

pub async fn mount_get_nonce_response_as_scoped(
    server: &MockServer,
    response: NonceResponse,
) -> MockGuard {
    prepare_get_nonce_response(response)
        .mount_as_scoped(server)
        .await
}

fn prepare_get_nonce_response(response: NonceResponse) -> Mock {
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
    .and(body_string_contains("accounts/nonce"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(wrapper)
            .append_header("Content-Type", "application/json"),
    )
    .expect(1)
}

pub async fn mount_tx_response(server: &MockServer, response: tx::Response) {
    prepare_tx_response(response).mount(server).await;
}

pub async fn mount_tx_response_as_scoped(server: &MockServer, response: tx::Response) -> MockGuard {
    prepare_tx_response(response).mount_as_scoped(server).await
}

fn prepare_tx_response(response: tx::Response) -> Mock {
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(serde_json::json!({"method": "tx"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&wrapper)
                .append_header("Content-Type", "application/json"),
        )
        .expect(1)
}

pub async fn mount_broadcast_tx_commit_response(
    server: &MockServer,
    response: tx_commit::Response,
) {
    prepare_broadcast_tx_commit_response(response)
        .mount(server)
        .await;
}

pub async fn mount_broadcast_tx_commit_response_as_scoped(
    server: &MockServer,
    response: tx_commit::Response,
) -> MockGuard {
    prepare_broadcast_tx_commit_response(response)
        .mount_as_scoped(server)
        .await
}

fn prepare_broadcast_tx_commit_response(response: tx_commit::Response) -> Mock {
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
}

/// Convert a `Request` object to a `SignedTransaction`
pub fn signed_tx_from_request(request: &wiremock::Request) -> SignedTransaction {
    use astria_core::generated::protocol::transactions::v1alpha1::SignedTransaction as RawSignedTransaction;
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
