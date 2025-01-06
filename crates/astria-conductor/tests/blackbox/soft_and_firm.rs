use std::time::Duration;

use astria_conductor::config::CommitLevel;
use futures::future::{
    join,
    join3,
};
use tokio::time::{
    sleep,
    timeout,
};

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
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn executes_soft_first_then_updates_firm() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
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
        Duration::from_millis(500),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "Conductor should have executed the block and updated the soft commitment state within \
         500ms",
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
        delay: Some(Duration::from_millis(500))
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
/// block, ensuring that update commitment state is not called upon receiving a tardy soft block.
/// Then, ensures the conductor updates the state for the soft block at the next height.
///
/// The following steps occur:
/// 1. Firm and soft blocks at the current height are mounted, the soft block with a 500ms delay to
///    allow for the firm block to be received first.
/// 2. The soft block for the next height is mounted with a 1000ms delay, so that execution and
///    state update of the current height happen before receipt of the next block.
/// 3. Mounts are made for firm and soft update commitment state calls, with the soft mount
///    expecting exactly 0 calls.
/// 4. 1000ms is allotted for the conductor to execute the block and update the firm commitment
///    state, noting that this allows time to test for an erroneously updated soft commitment state
///    before the conductor receives the next block.
/// 5. 2000ms is allotted for the conductor to execute the next block and update the soft commitment
///    state at the next height.
#[expect(
    clippy::too_many_lines,
    reason = "all mounts and test logic are necessary"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn executes_firm_then_soft_at_next_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
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

    // Mount soft block at current height with a slight delay
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
        delay: Duration::from_millis(500),
    );

    // Mount soft block at next height with substantial delay
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
        delay: Duration::from_millis(1000),
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

    // This guard's conditions will be checked when it is dropped, ensuring that there have been 0
    // calls to update the commitment state for the stale soft block. This is done instead of
    // waiting for the guard to be satisfied because if we call `wait_until_satisfied` on it, it
    // will succeed immediately and future erroneous calls will not be checked. It would be most
    // ideal to mount this logic directly to the server, but this workaround functions with the
    // current setup of the blackbox test helpers.
    let _stale_update_soft_commitment_state = mount_update_commitment_state!(
        test_conductor,
        mock_name: "should_be_ignored_update_commitment_state_soft",
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
        Duration::from_millis(2000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state_soft.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the block and updated the soft commitment state within \
         2000ms",
    );
}

#[expect(clippy::too_many_lines, reason = "it's a test, it's fine")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn missing_block_is_fetched_for_updating_firm_commitment() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
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

/// Tests if conductor restarts internal services if rollup shows signs of a restart.
///
/// Astria Geth will return a `PermissionDenied` error if the `execute_block` RPC is called
/// before `get_genesis_info` and `get_commitment_state` are called, which would happen in the
/// case of a restart. This response is mounted to cause the conductor to restart.
#[expect(
    clippy::too_many_lines,
    reason = "all lines fairly necessary, and I don't think a test warrants a refactor"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn conductor_restarts_on_permission_denied() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
        up_to_n_times: 2,
        halt_at_stop_height: false,
        expected_calls: 2,
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
        up_to_n_times: 2,
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
        delay: Some(Duration::from_millis(250))
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

/// Tests if the conductor correctly stops and procedes to restart after reaching the sequencer stop
/// height (from genesis info provided by rollup). In `SoftAndFirm` mode executor should not call
/// `execute_block` or `update_commitment_state` for any soft blocks at or above the stop height.
/// The conductor DOES call these on the firm block at the stop height, then proceeds to restart.
///
/// This test consists of the following steps:
/// 1. Mount commitment state and genesis info with a sequencer stop height of 3, only responding up
///    to 1 time so that Conductor will not receive the same response after restart.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer (soft blocks) for heights 3 and 4.
/// 4. Mount firm blocks at heights 3 and 4, with corresponding `update_commitment_state` mounts,
///    which should both be called. These are mounted with a slight delay to ensure that the soft
///    block arrives first after restart.
/// 5. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height
///    3. The soft block should not be executed since it is at the stop height, but the firm should
///    be.
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the firm block at
///    height 3 with a timeout of 1000ms. The test sleeps during this time, so that the following
///    mounts do not occur before the conductor restarts.
/// 7. Mount new genesis info with a sequencer stop height of 10 and a rollup start block height of
///    2, along with corresponding commitment state, reflecting that block 1 has already been
///    executed and the commitment state updated.
/// 8. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height 4
///    and await their satisfaction.
#[expect(
    clippy::too_many_lines,
    reason = "All lines reasonably necessary for the thoroughness of this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn conductor_restarts_after_reaching_stop_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 3,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
        up_to_n_times: 1, // We only respond once since this needs to be updated after restart
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
        up_to_n_times: 1, // We only respond once since this needs to be updated after restart
    );

    mount_sequencer_genesis!(test_conductor);
    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );
    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    // Mount soft blocks for heights 3 and 4
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    // Mount firm blocks for heights 3 and 4
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
        delay: Some(Duration::from_millis(200)) // short delay to ensure soft block at height 4 gets executed first after restart
    );
    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );
    mount_sequencer_commit!(
        test_conductor,
        height: 4u32,
    );
    mount_sequencer_validator_set!(test_conductor, height: 2u32);
    mount_sequencer_validator_set!(test_conductor, height: 3u32);

    let execute_block_1 = mount_executed_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
        expected_calls: 1, // This should not be called again after restart
    );

    // This should not be called, since it is at the sequencer stop height
    let _update_commitment_state_soft_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_1",
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

    let update_commitment_state_firm_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_1",
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
        expected_calls: 1, // Should not be called again after restart
    );

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_firm_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");

    // Wait until conductor is restarted before performing next set of mounts
    sleep(Duration::from_millis(1000)).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 1,
    );

    mount_get_commitment_state!(
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

    let execute_block_2 = mount_executed_block!(
        test_conductor,
        mock_name: "execute_block_2",
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
        expected_calls: 1,
    );

    let update_commitment_state_soft_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_2",
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
        expected_calls: 1,
    );

    let update_commitment_state_firm_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_2",
        firm: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        soft: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        base_celestia_height: 1,
        expected_calls: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join3(
            execute_block_2.wait_until_satisfied(),
            update_commitment_state_soft_2.wait_until_satisfied(),
            update_commitment_state_firm_2.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");
}

/// Tests if the conductor correctly stops and does not restart after reaching the sequencer stop
/// height if genesis info's `halt_at_stop_height` is `true`.
///
/// This test consists of the following steps:
/// 1. Mount commitment state and genesis info with a sequencer stop height of 3, expecting only 1
///    response.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer (soft blocks) for height 3.
/// 4. Mount firm blocks at height 3, with corresponding `update_commitment_state` mount.
/// 5. Mount `execute_block` and `update_commitment_state` for firm block at height
///    3. The soft block should not be executed since it is at the stop height, but the firmshould
///       be.
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the firm block
///    height 3 with a timeout of 1000ms.
/// 7. Allow ample time for the conductor to potentially restart erroneously.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn conductor_stops_at_stop_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 3,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
        up_to_n_times: 2, // allow for calls after an potential erroneous restart
        halt_at_stop_height: true,
        expected_calls: 1,
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
    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 3,
    );

    // Mount soft blocks for height 3
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    // Mount firm blocks for height 3
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

    let execute_block_1 = mount_executed_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state_firm_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_1",
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
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_firm_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");

    // Allow time for a potential erroneous restart
    sleep(Duration::from_millis(1000)).await;
}
