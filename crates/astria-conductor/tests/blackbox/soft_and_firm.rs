use std::time::Duration;

use astria_conductor::config::CommitLevel;
use futures::future::{
    join,
    join3,
};
use tokio::time::timeout;

use crate::{
    helpers::spawn_conductor,
    mount_abci_info,
    mount_celestia_blobs,
    mount_celestia_header_network_head,
    mount_execute_block_tonic_code,
    mount_executed_block,
    mount_get_block,
    mount_get_commitment_state,
    mount_get_filtered_sequencer_block,
    mount_get_genesis_info,
    mount_sequencer_commit,
    mount_sequencer_genesis,
    mount_sequencer_validator_set,
    mount_update_commitment_state,
    SEQUENCER_CHAIN_ID,
};

/// Tests if a single block is executed and the rollup's state updated (first soft, then firm).
///
/// The following steps are most important:
/// 1. a block at rollup number 1, sequencer height 2 is fetched from Sequencer
/// 2. the block is executed against the rollup
/// 3. the rollup's soft commitment state is updated to reflect the last execution
/// 4. block information for rollup number 1, sequencer height 2 is reconstructed from Celestia
///    height 1
/// 5. the rollup's firm commitment state is updated (but without executing the block)
///
/// NOTE: there is a potential race condition in this test in that the information could be first
/// retrieved from Celestia before Sequencer and executed against the rollup. In that case step 3.
/// would be skipped (no soft commitment update).
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
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

    mount_sequencer_genesis!(test_conductor, chain_id: SEQUENCER_CHAIN_ID);

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
        join3(
            execute_block.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
            update_commitment_state_firm.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the block and updated the soft and firm commitment states \
         within 1000ms",
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

    mount_sequencer_genesis!(test_conductor, chain_id: SEQUENCER_CHAIN_ID);

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

/// Tests if conductor restarts internal services if rollup shows signs of a restart.
///
/// Astria Geth will return a `PermissionDenied` error if the `execute_block` RPC is called
/// before `get_genesis_info` and `get_commitment_state` are called, which would happen in the
/// case of a restart. This response is mounted to cause the conductor to restart.
#[allow(clippy::too_many_lines)] // allow: all lines fairly necessary, and I don't think a test warrants a refactor
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn conductor_restarts_on_permission_denied() {
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

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    // mount tonic `PermissionDenied` error to cause the conductor to restart.
    // This mock can only be called up to 1 time, allowing a normal `execute_block` call after.
    let execute_block_tonic_code = mount_execute_block_tonic_code!(
        test_conductor,
        parent: [1; 64],
        status_code: tonic::Code::PermissionDenied,
    );

    timeout(
        Duration::from_millis(1000),
        execute_block_tonic_code.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have restarted after a permission denied error within 1000ms");

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
        join3(
            execute_block.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
            update_commitment_state_firm.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the block and updated the soft and firm commitment states \
         within 1000ms",
    );
}
