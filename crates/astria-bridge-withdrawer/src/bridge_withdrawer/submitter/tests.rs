use std::{
    io::Write as _,
    sync::Arc,
    time::Duration,
    vec,
};

use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    generated::protocol::account::v1alpha1::NonceResponse,
    primitive::v1::{
        asset::{
            self,
            default_native_asset,
            Denom,
        },
        Address,
        ASTRIA_ADDRESS_PREFIX,
    },
    protocol::{
        account::v1alpha1::AssetBalance,
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
        transaction::v1alpha1::{
            action::{
                BridgeUnlockAction,
                Ics20Withdrawal,
            },
            Action,
        },
    },
};
use astria_eyre::eyre::{
    self,
};
use ibc_types::core::client::Height as IbcHeight;
use once_cell::sync::Lazy;
use prost::Message as _;
use sequencer_client::{
    tendermint_rpc::{
        endpoint::broadcast::tx_commit,
        response,
    },
    SignedTransaction,
};
use serde_json::json;
use tempfile::NamedTempFile;
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
        broadcast::tx_sync,
        tx,
    },
    request,
};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::debug;
use wiremock::{
    matchers::{
        body_partial_json,
        body_string_contains,
    },
    Mock,
    MockGuard,
    MockServer,
    Request,
    ResponseTemplate,
};

use super::Submitter;
use crate::{
    bridge_withdrawer::{
        batch::Batch,
        ethereum::convert::BridgeUnlockMemo,
        startup,
        state,
        submitter,
    },
    metrics::Metrics,
};

const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";
const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .set_pretty_print(true)
            .filter_directives(&filter_directives)
            .try_init()
            .unwrap();
    } else {
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::sink)
            .try_init()
            .unwrap();
    }
});

struct TestSubmitter {
    submitter: Option<Submitter>,
    submitter_handle: submitter::Handle,
    startup_tx: Option<oneshot::Sender<startup::SubmitterInfo>>,
    cometbft_mock: MockServer,
    submitter_task_handle: Option<JoinHandle<Result<(), eyre::Report>>>,
}

impl TestSubmitter {
    async fn setup() -> Self {
        Lazy::force(&TELEMETRY);

        // set up external resources
        let shutdown_token = CancellationToken::new();

        // sequencer signer key
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(
                "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes(),
            )
            .unwrap();
        let sequencer_key_path = keyfile.path().to_str().unwrap().to_string();

        // cometbft
        let cometbft_mock = MockServer::start().await;
        let sequencer_cometbft_endpoint = format!("http://{}", cometbft_mock.address());

        // withdrawer state
        let state = Arc::new(state::State::new());
        // not testing watcher here so just set it to ready
        state.set_watcher_ready();
        let (startup_tx, startup_rx) = oneshot::channel();
        let startup_handle = startup::SubmitterHandle::new(startup_rx);

        let metrics = Box::leak(Box::new(Metrics::new()));

        let (submitter, submitter_handle) = submitter::Builder {
            shutdown_token: shutdown_token.clone(),
            startup_handle,
            sequencer_key_path,
            sequencer_cometbft_endpoint,
            state,
            metrics,
        }
        .build()
        .unwrap();

        Self {
            submitter: Some(submitter),
            submitter_task_handle: None,
            startup_tx: Some(startup_tx),
            submitter_handle,
            cometbft_mock,
        }
    }

    async fn startup(&mut self) {
        let submitter = self.submitter.take().unwrap();

        let mut state = submitter.state.subscribe();

        self.submitter_task_handle = Some(tokio::spawn(submitter.run()));

        self.startup_tx
            .take()
            .expect("should only send startup info once")
            .send(startup::SubmitterInfo {
                sequencer_chain_id: SEQUENCER_CHAIN_ID.to_string(),
            })
            .unwrap();

        // wait for the submitter to be ready
        state
            .wait_for(state::StateSnapshot::is_ready)
            .await
            .unwrap();
    }

    async fn spawn() -> Self {
        let mut submitter = Self::setup().await;
        submitter.startup().await;
        submitter
    }
}

async fn _register_default_chain_id_guard(cometbft_mock: &MockServer) -> MockGuard {
    _register_genesis_chain_id_response(SEQUENCER_CHAIN_ID, cometbft_mock).await
}

async fn _register_default_fee_asset_ids_guard(cometbft_mock: &MockServer) -> MockGuard {
    let fee_asset_ids = vec![default_native_asset().id()];
    _register_allowed_fee_asset_ids_response(fee_asset_ids, cometbft_mock).await
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

fn make_ics20_withdrawal_action() -> Action {
    let denom = DEFAULT_IBC_DENOM.parse::<Denom>().unwrap();
    let destination_chain_address = "address".to_string();
    let inner = Ics20Withdrawal {
        denom: denom.clone(),
        destination_chain_address,
        return_address: Address::builder()
            .array([0u8; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap(),
        amount: 99,
        memo: serde_json::to_string(&Ics20WithdrawalFromRollupMemo {
            memo: "hello".to_string(),
            bridge_address: crate::astria_address([0u8; 20]),
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT,
            transaction_hash: [2u8; 32],
        })
        .unwrap(),
        fee_asset_id: denom.id(),
        timeout_height: IbcHeight::new(u64::MAX, u64::MAX).unwrap(),
        timeout_time: 0, // zero this for testing
        source_channel: "channel-0".parse().unwrap(),
        bridge_address: None,
    };

    Action::Ics20Withdrawal(inner)
}

fn make_bridge_unlock_action() -> Action {
    let denom = default_native_asset();
    let inner = BridgeUnlockAction {
        to: Address::builder()
            .array([0u8; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap(),
        amount: 99,
        memo: serde_json::to_vec(&BridgeUnlockMemo {
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT.into(),
            transaction_hash: [1u8; 32].into(),
        })
        .unwrap(),
        fee_asset_id: denom.id(),
        bridge_address: None,
    };
    Action::BridgeUnlock(inner)
}

fn make_batch_with_bridge_unlock_and_ics20_withdrawal() -> Batch {
    Batch {
        actions: vec![make_ics20_withdrawal_action(), make_bridge_unlock_action()],
        rollup_height: 10,
    }
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

/// Convert a `Request` object to a `SignedTransaction`
fn signed_tx_from_request(request: &Request) -> SignedTransaction {
    use astria_core::generated::protocol::transaction::v1alpha1::SignedTransaction as RawSignedTransaction;
    use prost::Message as _;

    let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
        serde_json::from_slice(&request.body)
            .expect("deserialize to JSONRPC wrapped tx_sync::Request");
    let raw_signed_tx = RawSignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
        .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
        .expect("can't convert raw signed tx to checked signed tx");
    debug!(?signed_tx, "sequencer mock received signed transaction");

    signed_tx
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
    Mock::given(body_partial_json(json!({"method": "genesis"})))
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

async fn _register_allowed_fee_asset_ids_response(
    fee_asset_ids: Vec<asset::Id>,
    cometbft_mock: &MockServer,
) -> MockGuard {
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: astria_core::protocol::asset::v1alpha1::AllowedFeeAssetIdsResponse {
                fee_asset_ids,
                height: 1,
            }
            .into_raw()
            .encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = response::Wrapper::new_with_id(tendermint_rpc::Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({"method": "abci_query"})))
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
    Mock::given(body_partial_json(json!({"method": "abci_query"})))
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
    Mock::given(body_partial_json(json!({"method": "abci_query"})))
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
    Mock::given(body_partial_json(json!({"method": "abci_query"})))
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
    Mock::given(body_partial_json(json!({"method": "tx"})))
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

fn compare_actions(expected: &Action, actual: &Action) {
    match (expected, actual) {
        (Action::BridgeUnlock(expected), Action::BridgeUnlock(actual)) => {
            assert_eq!(expected, actual, "BridgeUnlock actions do not match");
        }
        (Action::Ics20Withdrawal(expected), Action::Ics20Withdrawal(actual)) => {
            assert_eq!(expected, actual, "Ics20Withdrawal actions do not match");
        }
        _ => panic!("Actions do not match"),
    }
}

/// Test that the submitter starts up successfully
#[tokio::test]
async fn submitter_startup_success() {
    let _submitter = TestSubmitter::spawn().await;
}

/// Sanity check to check that batch submission works
#[tokio::test]
async fn submitter_submit_success() {
    let submitter = TestSubmitter::spawn().await;
    let TestSubmitter {
        submitter_handle,
        cometbft_mock,
        ..
    } = submitter;

    // set up guards on mock cometbft
    let nonce_guard = register_get_nonce_response(
        &cometbft_mock,
        NonceResponse {
            height: 1,
            nonce: 0,
        },
    )
    .await;

    let broadcast_guard =
        register_broadcast_tx_commit_response(&cometbft_mock, make_tx_commit_success_response())
            .await;

    // send batch to submitter
    let batch = make_batch_with_bridge_unlock_and_ics20_withdrawal();
    submitter_handle.send_batch(batch).await.unwrap();

    // wait for nonce and broadcast guards to be satisfied
    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        broadcast_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // check the submitted transaction against the batch
    let requests = broadcast_guard.received_requests().await;
    assert_eq!(requests.len(), 1);
    let signed_transaction = signed_tx_from_request(&requests[0]);
    let actions = signed_transaction.actions();
    let expected_batch = make_batch_with_bridge_unlock_and_ics20_withdrawal();

    expected_batch
        .actions
        .iter()
        .zip(actions.iter())
        .for_each(|(expected, actual)| compare_actions(expected, actual));
}

/// Test that the submitter halts when transaction submissions fails to be included in the
/// mempool (CheckTx)
#[tokio::test]
async fn submitter_submit_check_tx_failure() {
    let submitter = TestSubmitter::spawn().await;
    let TestSubmitter {
        submitter_handle,
        cometbft_mock,
        mut submitter_task_handle,
        ..
    } = submitter;

    // set up guards on mock cometbft
    let nonce_guard = register_get_nonce_response(
        &cometbft_mock,
        NonceResponse {
            height: 1,
            nonce: 0,
        },
    )
    .await;

    let broadcast_guard = register_broadcast_tx_commit_response(
        &cometbft_mock,
        make_tx_commit_check_tx_failure_response(),
    )
    .await;

    // send batch to submitter
    let batch = make_batch_with_bridge_unlock_and_ics20_withdrawal();
    submitter_handle.send_batch(batch).await.unwrap();

    // wait for the nonce and broadcast guards to be satisfied
    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        broadcast_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // make sure the submitter halts and the task returns
    let _submitter_result = tokio::time::timeout(
        Duration::from_millis(100),
        submitter_task_handle.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
}

/// Test that the submitter halts when transaction submissions fails to be executed in a block
/// (DeliverTx)
#[tokio::test]
async fn submitter_submit_deliver_tx_failure() {
    let submitter = TestSubmitter::spawn().await;
    let TestSubmitter {
        submitter_handle,
        cometbft_mock,
        mut submitter_task_handle,
        ..
    } = submitter;

    // set up guards on mock cometbft
    let nonce_guard = register_get_nonce_response(
        &cometbft_mock,
        NonceResponse {
            height: 1,
            nonce: 0,
        },
    )
    .await;

    let broadcast_guard = register_broadcast_tx_commit_response(
        &cometbft_mock,
        make_tx_commit_deliver_tx_failure_response(),
    )
    .await;

    // send batch to submitter
    let batch = make_batch_with_bridge_unlock_and_ics20_withdrawal();
    submitter_handle.send_batch(batch).await.unwrap();

    // wait for the nonce and broadcast guards to be satisfied
    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        broadcast_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // make sure the submitter halts and the task returns
    let _submitter_result = tokio::time::timeout(
        Duration::from_millis(100),
        submitter_task_handle.take().unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
}
