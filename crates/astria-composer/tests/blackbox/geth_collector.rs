use std::time::Duration;

use astria_core::primitive::v1::RollupId;
use ethers::types::Transaction;

use crate::helper::{
    mount_broadcast_tx_sync_invalid_nonce_mock,
    mount_broadcast_tx_sync_mock,
    mount_matcher_verifying_tx_integrity,
    spawn_composer,
    TEST_ETH_TX_JSON,
};

#[tokio::test]
async fn tx_from_one_rollup_is_received_by_sequencer() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"], None, vec![], true).await;

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![0]).await;
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn collector_restarts_after_exit() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"], None, vec![], true).await;

    // get rollup node
    let rollup_node = test_composer.rollup_nodes.get("test1").unwrap();
    // abort the rollup node. The collector should restart after this abort
    rollup_node.cancel_subscriptions().unwrap();

    // FIXME: There is a race condition in the mock geth server between when the tx is pushed
    // and when the `eth_subscribe` task reads it.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // the collector will be restarted now, we should be able to send a tx normally
    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![0]).await;
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    // we added an extra 1000ms to the block time to make sure the collector has restarted
    // as the collector has to establish a new subscription on start up.
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms + 1000),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn invalid_nonce_causes_resubmission_under_different_nonce() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"], None, vec![], true).await;

    // Reject the first transaction for invalid nonce
    let invalid_nonce_guard = mount_broadcast_tx_sync_invalid_nonce_mock(
        &test_composer.sequencer,
        RollupId::from_unhashed_bytes("test1"),
    )
    .await;

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    // Expect nonce 1 again so that the resubmitted tx is accepted
    let valid_nonce_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![1]).await;

    // Mount a response of 1 to a nonce query
    test_composer
        .sequencer_mock
        .mount_pending_nonce_response(1, "setup correct nonce", 1)
        .await;

    // Push a tx to the rollup node so that it is picked up by the composer and submitted with the
    // stored nonce of 0, triggering the nonce refetch process
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        invalid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been rejected due to invalid nonce");

    tokio::time::timeout(
        Duration::from_millis(1000),
        valid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been accepted after nonce refetch");
}

#[tokio::test]
async fn nonce_taken_causes_resubmission_under_different_nonce() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"], None, vec![], true).await;

    // Reject the first transaction for taken nonce
    let invalid_nonce_guard = mount_broadcast_tx_sync_invalid_nonce_mock(
        &test_composer.sequencer,
        RollupId::from_unhashed_bytes("test1"),
    )
    .await;

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    // Expect nonce 1 again so that the resubmitted tx is accepted
    let valid_nonce_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![1]).await;

    // Mount a response of 1 to a nonce query
    test_composer
        .sequencer_mock
        .mount_pending_nonce_response(1, "setup correct nonce", 1)
        .await;

    // Push a tx to the rollup node so that it is picked up by the composer and submitted with the
    // stored nonce of 0, triggering the nonce refetch process
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms + 1000),
        invalid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been rejected due to invalid nonce");

    tokio::time::timeout(
        Duration::from_millis(1000),
        valid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been accepted after nonce refetch");
}

#[tokio::test]
async fn single_rollup_tx_payload_integrity() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"], None, vec![], true).await;

    let tx: Transaction = serde_json::from_str(TEST_ETH_TX_JSON).unwrap();
    let mock_guard =
        mount_matcher_verifying_tx_integrity(&test_composer.sequencer, tx.clone()).await;

    test_composer.rollup_nodes["test1"].push_tx(tx).unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms + 1000),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn current_pending_txs_are_submitted() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(
        &["test1"],
        None,
        vec![Transaction::default(), Transaction::default()],
        true,
    )
    .await;

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    // Only checking for nonce 0 since both eth transactions are bundled
    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![0]).await;

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}
