use std::time::Duration;

use astria_core::{
    generated::astria::composer::v1::{
        grpc_collector_service_client::GrpcCollectorServiceClient,
        SubmitRollupTransactionRequest,
    },
    primitive::v1::RollupId,
};
use ethers::prelude::Transaction;
use prost::bytes::Bytes;

use crate::helper::{
    mount_broadcast_tx_sync_invalid_nonce_mock,
    mount_broadcast_tx_sync_mock,
    mount_matcher_verifying_tx_integrity,
    spawn_composer,
    TEST_ETH_TX_JSON,
};

#[tokio::test]
async fn tx_from_one_rollup_is_received_by_sequencer() {
    let test_composer = spawn_composer(&[], None, true).await;
    let rollup_id = RollupId::from_unhashed_bytes("test1");
    let expected_chain_ids = vec![rollup_id];
    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_chain_ids, vec![0]).await;

    let tx = Transaction::default();
    // send sequence action request to the grpc collector
    let mut composer_client = GrpcCollectorServiceClient::connect(format!(
        "http://{}",
        test_composer.grpc_collector_addr
    ))
    .await
    .unwrap();
    composer_client
        .submit_rollup_transaction(SubmitRollupTransactionRequest {
            rollup_id: Some(rollup_id.into_raw()),
            data: Bytes::copy_from_slice(&tx.rlp()),
        })
        .await
        .expect("rollup transactions should have been submitted successfully to grpc collector");

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn invalid_nonce_causes_resubmission_under_different_nonce() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let rollup_id = RollupId::from_unhashed_bytes("test1");
    let test_composer = spawn_composer(&[], None, true).await;

    // Reject the first transaction for invalid nonce
    let invalid_nonce_guard =
        mount_broadcast_tx_sync_invalid_nonce_mock(&test_composer.sequencer, rollup_id).await;

    // Mount a response of 1 to a nonce query
    test_composer
        .sequencer_mock
        .mount_pending_nonce_response(1, "setup correct nonce", 1)
        .await;

    let expected_chain_ids = vec![rollup_id];
    // Expect nonce 1 again so that the resubmitted tx is accepted
    let valid_nonce_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_chain_ids, vec![1]).await;

    // Send a tx to the composer so that it is picked up by the grpc collector and submitted with
    // the stored nonce of 0, triggering the nonce refetch process
    let tx = Transaction::default();
    // send sequence action request to the grpc collector
    let mut composer_client = GrpcCollectorServiceClient::connect(format!(
        "http://{}",
        test_composer.grpc_collector_addr
    ))
    .await
    .unwrap();
    composer_client
        .submit_rollup_transaction(SubmitRollupTransactionRequest {
            rollup_id: Some(rollup_id.into_raw()),
            data: Bytes::copy_from_slice(&tx.rlp()),
        })
        .await
        .expect("rollup transactions should have been submitted successfully to grpc collector");

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        invalid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been rejected due to invalid nonce");

    tokio::time::timeout(
        Duration::from_millis(100),
        valid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been accepted after nonce refetch");
}

#[tokio::test]
async fn single_rollup_tx_payload_integrity() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let rollup_id = RollupId::from_unhashed_bytes("test1");
    let test_composer = spawn_composer(&[], None, true).await;

    let tx: Transaction = serde_json::from_str(TEST_ETH_TX_JSON).unwrap();
    let mock_guard =
        mount_matcher_verifying_tx_integrity(&test_composer.sequencer, tx.clone()).await;

    // send sequence action request to the grpc generic collector
    let mut composer_client = GrpcCollectorServiceClient::connect(format!(
        "http://{}",
        test_composer.grpc_collector_addr
    ))
    .await
    .unwrap();
    composer_client
        .submit_rollup_transaction(SubmitRollupTransactionRequest {
            rollup_id: Some(rollup_id.into_raw()),
            data: Bytes::copy_from_slice(&tx.rlp()),
        })
        .await
        .expect("rollup transactions should have been submitted successfully to grpc collector");

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect(
        "mocked sequencer should have received a broadcast message containing expected payload \
         from composer",
    );
}
