use std::time::Duration;

use astria_conductor::{
    conductor::InitializationError,
    config::CommitLevel,
};
use futures::future::{
    join,
    join4,
};
use tokio::time::timeout;

use crate::{
    helpers::spawn_conductor,
    mount_abci_info,
    mount_executed_block,
    mount_get_commitment_state,
    mount_get_filtered_sequencer_block,
    mount_get_genesis_info,
    mount_sequencer_genesis,
    mount_update_commitment_state,
};

pub const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_sequencer_genesis!(
        test_conductor,
        chain_id: SEQUENCER_CHAIN_ID,
    );

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 1,
        celestia_block_variance: 10,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn submits_two_heights_in_succession() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_sequencer_genesis!(
        test_conductor,
        chain_id: SEQUENCER_CHAIN_ID,
    );

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 1,
        celestia_block_variance: 10,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    let execute_block_number_2 = mount_executed_block!(
        test_conductor,
        mock_name: "first_execute",
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state_number_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "first_update",
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    let execute_block_number_3 = mount_executed_block!(
        test_conductor,
        mock_name: "second_execute",
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
    );

    let update_commitment_state_number_3 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "second_update",
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        base_celestia_height: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join4(
            execute_block_number_2.wait_until_satisfied(),
            update_commitment_state_number_2.wait_until_satisfied(),
            execute_block_number_3.wait_until_satisfied(),
            update_commitment_state_number_3.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn skips_already_executed_heights() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_sequencer_genesis!(
        test_conductor,
        chain_id: SEQUENCER_CHAIN_ID,
    );

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 1,
        celestia_block_variance: 10,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 5,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 7,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 7,
    );

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 6,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 6,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn requests_from_later_genesis_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_sequencer_genesis!(
        test_conductor,
        chain_id: SEQUENCER_CHAIN_ID,
    );

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 10,
        celestia_block_variance: 10,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 12,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 12,
    );

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1
    );

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn exits_on_sequencer_chain_id_mismatch() {
    let mut test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_sequencer_genesis!(
        test_conductor,
        chain_id: "bad_chain_id",
    );

    if let Some(task_handle) = test_conductor.conductor.task.take() {
        match task_handle.await {
            Ok(Ok(())) => panic!("conductor should have exited with an error, no error received"),
            Ok(Err(e)) => match e.downcast_ref::<InitializationError>() {
                Some(InitializationError::WrongSequencerChainID {
                    expected,
                    actual,
                }) => {
                    assert_eq!(expected, SEQUENCER_CHAIN_ID);
                    assert_eq!(actual, "bad_chain_id");
                }
                _ => panic!(
                    "conductor should have exited with a WrongSequencerChainID error, received \
                     error {e}"
                ),
            },
            Err(e) => panic!("conductor handle resulted in an error: {e}"),
        };
    } else {
        panic!("no handle found for conductor tasks");
    }
}
