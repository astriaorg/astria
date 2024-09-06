use std::time::Duration;

use astria_core::{
    generated::protocol::accounts::v1alpha1::NonceResponse,
    primitive::v1::RollupId,
};
use ethers::types::Transaction;
use futures::future::join;
use futures::join;
use astria_composer::{mount_executed_block, mount_get_commitment_state};
use astria_core::protocol::transaction::v1alpha1::action::SequenceAction;
use astria_core::sequencerblock::v1alpha1::block::RollupData;

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
    let test_composer = spawn_composer("test1").await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("composer and sequencer should have been setup successfully");

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![0]).await;

    let tx = Transaction::default();
    let data = tx.rlp().to_vec();
    let rollup_data = vec![RollupData::SequencedData(data).to_raw()];

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    let test_executor = test_composer.test_executor;

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    test_composer.rollup_nodes["test1"]
        .push_tx(tx)
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        join(
            mock_guard.wait_until_satisfied(),
            execute_block.wait_until_satisfied()
        )
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn collector_restarts_after_exit() {
    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer("test1").await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("composer and sequencer should have been setup successfully");

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

    let test_executor = test_composer.test_executor;

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let tx = Transaction::default();
    let data = tx.rlp().to_vec();
    let rollup_data = vec![RollupData::SequencedData(data).to_raw()];

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    test_composer.rollup_nodes["test1"]
        .push_tx(tx)
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
    use crate::helper::mock_sequencer::mount_abci_query_mock;

    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer("test1").await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("composer and sequencer should have been setup successfully");

    // Reject the first transaction for invalid nonce
    let invalid_nonce_guard = mount_broadcast_tx_sync_invalid_nonce_mock(
        &test_composer.sequencer,
        RollupId::from_unhashed_bytes("test1"),
    )
    .await;

    // Mount a response of 0 to a nonce query
    let nonce_refetch_guard = mount_abci_query_mock(
        &test_composer.sequencer,
        "accounts/nonce",
        NonceResponse {
            height: 0,
            nonce: 1,
        },
    )
    .await;

    let expected_rollup_ids = vec![RollupId::from_unhashed_bytes("test1")];
    // Expect nonce 1 again so that the resubmitted tx is accepted
    let valid_nonce_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, expected_rollup_ids, vec![1]).await;

    let test_executor = test_composer.test_executor;

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let tx = Transaction::default();
    let data = tx.rlp().to_vec();
    let rollup_data = vec![RollupData::SequencedData(data).to_raw()];

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    // Push a tx to the rollup node so that it is picked up by the composer and submitted with the
    // stored nonce of 0, triggering the nonce refetch process
    test_composer.rollup_nodes["test1"]
        .push_tx(tx)
        .unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        invalid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("sequencer tx should have been rejected due to invalid nonce");

    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_refetch_guard.wait_until_satisfied(),
    )
    .await
    .expect("new nonce should have been fetched from the sequencer");

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
    let test_composer = spawn_composer("test1").await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("composer and sequencer should have been setup successfully");

    let tx: Transaction = serde_json::from_str(TEST_ETH_TX_JSON).unwrap();
    let mock_guard =
        mount_matcher_verifying_tx_integrity(&test_composer.sequencer, tx.clone()).await;

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    let test_executor = test_composer.test_executor;

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let data = tx.rlp().to_vec();
    let rollup_data = vec![RollupData::SequencedData(data).to_raw()];

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    test_composer.rollup_nodes["test1"].push_tx(tx).unwrap();

    // wait for 1 sequencer block time to make sure the bundle is preempted
    tokio::time::timeout(
        Duration::from_millis(test_composer.cfg.block_time_ms),
        join(
            mock_guard.wait_until_satisfied(),
            execute_block.wait_until_satisfied()
        )
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}
