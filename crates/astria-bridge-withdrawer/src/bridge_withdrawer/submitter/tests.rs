use std::{
    io::Write as _,
    sync::Arc,
    time::Duration,
    vec,
};

use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    generated::protocol::account::v1alpha1::NonceResponse,
    primitive::v1::asset,
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
use tokio::task::JoinHandle;
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
