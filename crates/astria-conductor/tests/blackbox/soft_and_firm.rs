use std::time::Duration;

use astria_conductor::config::CommitLevel;
use futures::future::{
    join,
    join3,
};
use tokio::time::timeout;

use crate::{
    celestia_network_head,
    commitment_state,
    genesis_info,
    helpers::spawn_conductor,
    mount_celestia_blobs,
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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .mount()
        .await;

    test_conductor.mock_abci_info(3).mount().await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;

    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;

    let execute_block = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state_soft = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft")
        .mount_as_scoped()
        .await;

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

    test_conductor.mock_sequencer_commit(3).mount().await;

    test_conductor.mock_validator_set(2).mount().await;

    let update_commitment_state_firm = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm")
        .mount_as_scoped()
        .await;

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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .mount()
        .await;

    test_conductor.mock_abci_info(4).mount().await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    test_conductor.mock_sequencer_commit(3).mount().await;

    test_conductor.mock_validator_set(2).mount().await;

    let execute_block = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    // Mount soft block at current height with a slight delay
    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .named("get_filtered_sequencer_block_1")
        .delay(Duration::from_millis(500))
        .mount()
        .await;

    // Mount soft block at next height with substantial delay
    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .named("get_filtered_sequencer_block_2")
        .delay(Duration::from_millis(1000))
        .mount()
        .await;

    let update_commitment_state_firm = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm")
        .mount_as_scoped()
        .await;

    // This guard's conditions will be checked when it is dropped, ensuring that there have been 0
    // calls to update the commitment state for the stale soft block. This is done instead of
    // waiting for the guard to be satisfied because if we call `wait_until_satisfied` on it, it
    // will succeed immediately and future erroneous calls will not be checked. It would be most
    // ideal to mount this logic directly to the server, but this workaround functions with the
    // current setup of the blackbox test helpers.
    let _stale_update_soft_commitment_state = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("should_be_ignored_update_commitment_state_soft")
        .expect(0)
        .mount_as_scoped()
        .await;

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

    let execute_block = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state_soft = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft")
        .mount_as_scoped()
        .await;

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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .mount()
        .await;

    test_conductor.mock_abci_info(4).mount().await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor
        .mock_get_block(2, [2; 64], [1; 64])
        .mount()
        .await;

    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    test_conductor.mock_sequencer_commit(3).mount().await;

    test_conductor.mock_validator_set(2).mount().await;

    let update_commitment_state_firm = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm")
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        update_commitment_state_firm.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have confirmed the pending block within 1000ms");

    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .mount()
        .await;

    let execute_block_soft = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .named("execute_block_soft")
        .mount_as_scoped()
        .await;

    let update_commitment_state_soft = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft")
        .mount_as_scoped()
        .await;

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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .up_to_n_times(2)
        .expect(2)
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .up_to_n_times(2)
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
        delay: Some(Duration::from_millis(250))
    );

    test_conductor.mock_sequencer_commit(3).mount().await;

    test_conductor.mock_validator_set(2).mount().await;

    test_conductor.mock_abci_info(3).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;

    // mount tonic `PermissionDenied` error to cause the conductor to restart.
    // This mock can only be called up to 1 time, allowing a normal `execute_block` call after.
    let execute_block_tonic_code = test_conductor
        .mock_execute_block_status_code([1; 64], tonic::Code::PermissionDenied)
        .up_to_n_times(1)
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        execute_block_tonic_code.wait_until_satisfied(),
    )
    .await
    .expect("conductor should have restarted after a permission denied error within 1000ms");

    let execute_block = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state_soft = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft")
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm")
        .mount_as_scoped()
        .await;

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
/// 1. Mount commitment state and genesis info with a sequencer stop height of 3, only responding up
///    to 1 time so that Conductor will not receive the same response after restart.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer blocks (soft blocks) for heights 3 and 4.
/// 4. Mount firm blocks at heights 3 and 4 with a slight delay to ensure that the soft blocks
///    arrive first.
/// 5. Mount `execute_block` and `update_commitment_state` for both soft and firm blocks at height 3
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the soft and firm
///    blocks at height 3 with a timeout of 1000ms.
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
async fn restarts_after_reaching_soft_stop_height_first() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 2,
        ))
        .named("get_genesis_info_1")
        .up_to_n_times(1) // We only respond once since this needs to be updated after restart
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .named("get_commitment_state_1")
        .up_to_n_times(1) // We only respond once since this needs to be updated after restart
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;
    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;
    test_conductor.mock_abci_info(4).mount().await;

    // Mount soft blocks for heights 3 and 4
    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .named("get_filtered_sequencer_block_1")
        .mount()
        .await;
    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .named("get_filtered_sequencer_block_2")
        .mount()
        .await;

    // Mount firm blocks for heights 3 and 4
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
        delay: Some(Duration::from_millis(200)) // short delay to ensure soft block at height 4 gets executed first after restart
    );
    test_conductor.mock_sequencer_commit(3).mount().await;
    test_conductor.mock_sequencer_commit(4).mount().await;
    test_conductor.mock_validator_set(2).mount().await;
    test_conductor.mock_validator_set(3).mount().await;

    let execute_block_1 = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .named("execute_block_1")
        .expect(1) // This should not be called again after restart
        .mount_as_scoped()
        .await;

    let update_commitment_state_soft_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft_1")
        .expect(1)
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm_1")
        .expect(1)
        .mount_as_scoped()
        .await;

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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 4,
            celestia_block_variance: 10,
            rollup_start_block_number: 3,
            rollup_stop_block_number: 9,
        ))
        .named("get_genesis_info_2")
        .up_to_n_times(1)
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .named("get_commitment_state_2")
        .mount()
        .await;

    let execute_block_2 = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .named("execute_block_2")
        .expect(1)
        .mount_as_scoped()
        .await;

    // This condition should be satisfied, since there is a delay on the firm block response
    let update_commitment_state_soft_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft_2")
        .expect(1)
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm_2")
        .expect(1)
        .mount_as_scoped()
        .await;

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
/// 1. Mount commitment state and genesis info with a sequencer stop height of 3, only responding up
///    to 1 time so that Conductor will not receive the same response after restart.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer blocks (soft blocks) for heights 3 and 4 with a slight delay,
///    to ensure the firm blocks arrive first.
/// 4. Mount firm blocks at heights 3 and 4.
/// 5. Mount `update_commitment_state` for the soft block at height 3, expecting 0 calls since the
///    firm block will be received first.
/// 5. Mount `execute_block` and `update_commitment_state` for firm block at height 3.
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the firm block at
///    height 3 with a timeout of 1000ms.
/// 7. Mount new genesis info with a sequencer stop height of 10 and a rollup start block height of
///    2, along with corresponding commitment state, reflecting that block 1 has already been
///    executed and the commitment state updated.
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

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 2,
        ))
        .named("get_genesis_info_1")
        .up_to_n_times(1) // We only respond once since this needs to be updated after restart
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .named("get_commitment_state_1")
        .up_to_n_times(1) // We only respond once since this needs to be updated after restart
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;
    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;
    test_conductor.mock_abci_info(4).mount().await;

    // Mount soft blocks for heights 3 and 4
    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .named("get_filtered_sequencer_block_1")
        .delay(Duration::from_millis(200))
        .mount()
        .await;
    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .named("get_filtered_sequencer_block_2")
        .mount()
        .await;

    // Mount firm blocks for heights 3 and 4
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
    );
    test_conductor.mock_sequencer_commit(3).mount().await;
    test_conductor.mock_sequencer_commit(4).mount().await;
    test_conductor.mock_validator_set(2).mount().await;
    test_conductor.mock_validator_set(3).mount().await;

    let execute_block_1 = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .named("execute_block_1")
        .expect(1) // This should not be called again after restart
        .mount_as_scoped()
        .await;

    // Should not be called since the firm block will be received first
    let _update_commitment_state_soft_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("should_not_be_called_update_commitment_state_soft_1")
        .expect(0)
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm_1")
        .expect(1)
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_firm_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 4,
            celestia_block_variance: 10,
            rollup_start_block_number: 3,
            rollup_stop_block_number: 9,
        ))
        .named("get_genesis_info_2")
        .up_to_n_times(1)
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .named("get_commitment_state_2")
        .mount()
        .await;

    let execute_block_2 = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .named("execute_block_2")
        .expect(1)
        .mount_as_scoped()
        .await;

    // This condition does not need to be satisfied, since firm block may fire first after restart
    let _update_commitment_state_soft_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft_2")
        .expect(0..=1)
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm_2")
        .expect(1)
        .mount_as_scoped()
        .await;

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

/// Tests if the conductor correctly stops and does not restart after reaching the sequencer stop
/// height if genesis info's `halt_at_rollup_stop_number` is `true`.
///
/// This test consists of the following steps:
/// 1. Mount commitment state and genesis info with a sequencer stop height of 3, expecting only 1
///    response.
/// 2. Mount Celestia network head and sequencer genesis.
/// 3. Mount ABCI info and sequencer blocks (soft blocks) for height 3.
/// 4. Mount firm blocks at height 3.
/// 5. Mount `execute_block` and `update_commitment_state` for soft and firm blocks at height 3.
/// 6. Await satisfaction of the `execute_block` and `update_commitment_state` for the firm block
///    height 3 with a timeout of 1000ms. The soft mount need not be satisfied in the case that the
///    firm block is received first. The test case of ensuring the soft commitment state is updated
///    correctly in the case of receiving a soft block first is covered in
///    `conductor_restarts_after_reaching_soft_stop_height_first`.
/// 7. Allow ample time for the conductor to potentially restart erroneously.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn stops_at_stop_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 2,
            halt_at_rollup_stop_number: true,
        ))
        .up_to_n_times(2) // allow for calls after a potential erroneous restart
        .expect(1)
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
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
        ))
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;
    test_conductor
        .mock_celestia_header_network_head(celestia_network_head!(height: 1u32))
        .mount()
        .await;
    test_conductor.mock_abci_info(3).mount().await;

    // Mount soft blocks for height 3
    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;

    // Mount firm blocks for height 3
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );
    test_conductor.mock_sequencer_commit(3).mount().await;
    test_conductor.mock_validator_set(2).mount().await;

    let execute_block_1 = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .named("execute_block_1")
        .mount_as_scoped()
        .await;

    let _update_commitment_state_soft_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_soft_1")
        .expect(0..=1)
        .mount_as_scoped()
        .await;

    let update_commitment_state_firm_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
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
        ))
        .named("update_commitment_state_firm_1")
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_firm_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect("conductor should have updated the firm commitment state within 1000ms");
}
