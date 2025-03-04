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
    mount_create_execution_session,
    mount_execute_block,
    mount_execute_block_tonic_code,
    mount_get_executed_block_metadata,
    mount_get_filtered_sequencer_block,
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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
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

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
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

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
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
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 3,
        hash: "3",
        parent: "2",
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            lowest_celestia_search_height: 1,
        )
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_sequencer_genesis!(test_conductor);

    mount_get_executed_block_metadata!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
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
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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

    let execute_block_soft = mount_execute_block!(
        test_conductor,
        number: 3,
        hash: "3",
        parent: "2",
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
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
async fn restarts_on_permission_denied() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
        expected_calls: 2,
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
        parent: "1",
        status_code: tonic::Code::PermissionDenied,
    );

    timeout(
        Duration::from_millis(1000),
        execute_block_tonic_code.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have restarted after a permission denied error within 1000ms");

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state_soft = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
    );

    let update_commitment_state_firm = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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

/// Tests if the conductor correctly stops and procedes to restart after soft block height reaches
/// sequencer stop height (from genesis info provided by rollup). In `SoftAndFirm` mode executor
/// should execute both the soft and firm blocks at the stop height and then perform a restart.
///
/// This test consists of the following steps:
/// 1. Mount execution session with a rollup stop number of 2 (sequencer height 3), only responding
///    up to 1 time so that Conductor will not receive the same response after restart.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer blocks (soft blocks) for heights 3 and 4.
/// 4. Mount firm blocks at heights 3 and 4 with a slight delay to ensure that the soft blocks
///    arrive first.
/// 5. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height 3
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the soft and firm
///    blocks at height 3 with a timeout of 1000ms.
/// 7. Mount new execution session with a rollup stop number of 9 and a start block number of 2,
///    reflecting that block 1 has already been executed and the commitment state updated.
/// 8. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height 4
///    and await their satisfaction.
#[expect(
    clippy::too_many_lines,
    reason = "All lines reasonably necessary for the thoroughness of this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restarts_after_reaching_soft_stop_height_first() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 2,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
        up_to_n_times: 1, // We only respond once since a new execution session is needed after restart
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
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
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

    let execute_block_1 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: "2",
        parent: "1",
        expected_calls: 1, // This should not be called again after restart
    );

    let update_commitment_state_soft_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_1",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

    let update_commitment_state_firm_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_1",
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1, // Should not be called again after restart
    );

    timeout(
        Duration::from_millis(1000),
        join3(
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_firm_1.wait_until_satisfied(),
            update_commitment_state_soft_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 3,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 4,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            soft: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            lowest_celestia_search_height: 1,
        ),
    );

    let execute_block_2 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_2",
        number: 3,
        hash: "3",
        parent: "2",
        expected_calls: 1,
    );

    // This condition should be satisfied, since there is a delay on the firm block response
    let update_commitment_state_soft_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_2",
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

    let update_commitment_state_firm_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_2",
        firm: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join3(
            execute_block_2.wait_until_satisfied(),
            update_commitment_state_firm_2.wait_until_satisfied(),
            update_commitment_state_soft_2.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");
}

/// Tests if the conductor correctly stops and procedes to restart after firm height reaches
/// sequencer stop height, *without* updating soft commitment state, since the firm was received
/// first.
///
/// This test consists of the following steps:
/// 1. Mount execution session with a rollup stop number of 2 (sequencer height 3), only responding
///    up to 1 time so that Conductor will not receive the same response after restart.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer blocks (soft blocks) for heights 3 and 4 with a slight delay,
///    to ensure the firm blocks arrive first.
/// 4. Mount firm blocks at heights 3 and 4.
/// 5. Mount `update_commitment_state` for the soft block at height 3, expecting 0 calls since the
///    firm block will be received first.
/// 5. Mount `execute_block` and `update_commitment_state` for firm block at height 3.
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the firm block at
///    height 3 with a timeout of 1000ms.
/// 7. Mount new genesis info with a rollup stop number of 9 and a start block number of 2,
///    reflecting that block 1 has already been executed and the commitment state updated.
/// 8. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height 4
///    and await their satisfaction (the soft mount need not be satisfied in the case that the firm
///    block is received first; we are just looking to see that the conductor restarted properly).
#[expect(
    clippy::too_many_lines,
    reason = "All lines reasonably necessary for the thoroughness of this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restarts_after_reaching_firm_stop_height_first() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 2,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
        up_to_n_times: 1, // We only respond once since a new execution session is needed after restart
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
        delay: Duration::from_millis(200),
    );
    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    // Mount firm blocks for heights 3 and 4
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
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

    let execute_block_1 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: "2",
        parent: "1",
        expected_calls: 1, // This should not be called again after restart
    );

    // Should not be called since the firm block will be received first
    let _update_commitment_state_soft_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_1",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 0,
    );

    let update_commitment_state_firm_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_1",
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 3,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 4,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            soft: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            lowest_celestia_search_height: 1,
        ),
    );

    let execute_block_2 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_2",
        number: 3,
        hash: "3",
        parent: "2",
        expected_calls: 1,
    );

    // This condition does not need to be satisfied, since firm block may fire first after restart
    let _update_commitment_state_soft_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_soft_2",
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 0..=1,
    );

    let update_commitment_state_firm_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_firm_2",
        firm: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_2.wait_until_satisfied(),
            update_commitment_state_firm_2.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");
}
