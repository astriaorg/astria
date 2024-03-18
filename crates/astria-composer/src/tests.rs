use std::collections::HashMap;

use astria_core::sequencer::v1::{
    asset::default_native_asset_id,
    transaction::action::SequenceAction,
    RollupId,
};
use ethers::types::Transaction;
use tokio_util::task::JoinMap;

use crate::{
    collectors::{
        geth::Status,
        Geth,
    },
    composer::reconnect_exited_collector,
};

/// This tests the `reconnect_exited_collector` handler.
#[tokio::test]
async fn collector_is_reconnected_after_exit() {
    let mock_geth = test_utils::mock::Geth::spawn().await;
    let rollup_name = "test".to_string();
    let rollup_url = format!("ws://{}", mock_geth.local_addr());
    let rollups = HashMap::from([(rollup_name.clone(), rollup_url.clone())]);

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    let mut collector_tasks = JoinMap::new();
    let collector = Geth::new(rollup_name.clone(), rollup_url.clone(), tx.clone());
    let mut status = collector.subscribe();
    collector_tasks.spawn(rollup_name.clone(), collector.run_until_stopped());
    status.wait_for(Status::is_connected).await.unwrap();
    let rollup_tx = Transaction::default();
    let expected_seq_action = SequenceAction {
        rollup_id: RollupId::from_unhashed_bytes(&rollup_name),
        data: Transaction::default().rlp().to_vec(),
        fee_asset_id: default_native_asset_id(),
    };
    let _ = mock_geth.push_tx(rollup_tx.clone()).unwrap();
    let collector_tx = rx.recv().await.unwrap();

    assert_eq!(
        RollupId::from_unhashed_bytes(&rollup_name),
        collector_tx.rollup_id,
    );
    assert_eq!(expected_seq_action.data, collector_tx.data);

    let _ = mock_geth.abort().unwrap();

    let (exited_rollup_name, exit_result) = collector_tasks.join_next().await.unwrap();
    assert_eq!(exited_rollup_name, rollup_name);
    assert!(collector_tasks.is_empty());

    // after aborting pushing a new tx to subscribers should fail as there are no broadcast
    // receivers
    assert!(mock_geth.push_tx(Transaction::default()).is_err());

    let mut statuses = HashMap::new();
    reconnect_exited_collector(
        &mut statuses,
        &mut collector_tasks,
        tx.clone(),
        &rollups,
        rollup_name.clone(),
        exit_result,
    );

    assert!(collector_tasks.contains_key(&rollup_name));
    statuses
        .get_mut(&rollup_name)
        .unwrap()
        .wait_for(Status::is_connected)
        .await
        .unwrap();
    let _ = mock_geth.push_tx(rollup_tx).unwrap();
    let collector_tx = rx.recv().await.unwrap();

    assert_eq!(
        RollupId::from_unhashed_bytes(&rollup_name),
        collector_tx.rollup_id,
    );
    assert_eq!(expected_seq_action.data, collector_tx.data);
}
