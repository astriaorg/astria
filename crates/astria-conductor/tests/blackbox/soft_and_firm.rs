use std::time::Duration;

use astria_conductor::config::CommitLevel;
use futures::future::join;
use tokio::time::timeout;

use crate::{
    helpers::spawn_conductor,
    mount_abci_info,
    mount_celestia_blobs,
    mount_celestia_header_network_head,
    mount_executed_block,
    mount_get_block,
    mount_get_commitment_state,
    mount_get_filtered_sequencer_block,
    mount_get_genesis_info,
    mount_sequencer_commit,
    mount_sequencer_genesis,
    mount_sequencer_validator_set,
    mount_update_commitment_state,
};

/// Tests if a single block is executed and the rollup's state updated (first firm, then soft is
/// ignored).
///
/// This is to test for the unlikely scenario of receiving a firm block from Celestia before the
/// soft block from sequencer.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_firm_only() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

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

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let update_commitment_state_firm = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
            update_commitment_state_firm.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the block and updated the firm commitment state within \
         1000ms",
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft",
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
        expected_calls: 0,
    );

    timeout(
        Duration::from_millis(500),
        update_commitment_state_soft.wait_until_satisfied(),
    )
    .await
    .expect_err("conductor should not have updated firm commitment state.");
}

/// Tests if a single block is executed and the rollup's state updated (first soft, then firm).
///
/// The following steps are most important:
/// 1. a block at rollup number 1, sequencer height 2 is fetched from Sequencer
/// 2. the block is executed against the rollup
/// 3. the rollup's soft commitment state is updated to reflect the last execution
/// 4. block information for rollup number 1, sequencer height 2 is reconstructed from Celestia
///    height 1
/// 5. the rollup's firm commitment state is updated (but without executing the block)
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple_soft_first() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

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

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
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

    let update_commitment_state_soft = mount_update_commitment_state!(
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
            update_commitment_state_soft.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "Conductor should have executed the block and updated the soft commitment state within \
         1000ms",
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let update_commitment_state_firm = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
        update_commitment_state_firm.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");
}

/// Tests if a single block is executed and the rollup's state updated after first receiving a firm
/// block, then for the soft block at the next height.
#[allow(clippy::too_many_lines)] // allow: all mounts and test logic are necessary.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn executes_firm_then_soft_at_next_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

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

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state_firm = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
            update_commitment_state_firm.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "Conductor should have executed the block and updated the firm commitment state within \
         1000ms",
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the block and updated the soft commitment state within \
         1000ms",
    );
}

#[allow(clippy::too_many_lines)] // it's a test, it's fine
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn missing_block_is_fetched_for_updating_firm_commitment() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

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
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_sequencer_genesis!(test_conductor);

    mount_get_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let update_commitment_state_firm = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
        update_commitment_state_firm.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have confirmed the pending block within 1000ms");

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    let execute_block_soft = mount_executed_block!(
        test_conductor,
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
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
        join(
            execute_block_soft.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}
