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
    mount_executed_block,
    mount_get_commitment_state,
    mount_get_filtered_sequencer_block,
    mount_get_genesis_info,
    mount_pending_blocks,
    mount_sequencer_commit,
    mount_sequencer_genesis,
    mount_sequencer_validator_set,
    mount_update_commitment_state,
};

/// Tests if a single block is executed and the rollup's state updated (first soft, then firm).
///
/// The following steps are most important:
/// 1. a block at rollup number 1, sequencer height 2 is fetched from Sequencer
/// 2. the block is executed against the rollup
/// 3. the rollup's soft commitment state is updated to reflect the last exection
/// 4. block information for rollup number 1, sequencer height 2 is reconstructed from Celestia
///    height 1
/// 5. the rollup's firm commitment state is updated (but without executing the block)
///
/// NOTE: there is a potential race condition in this test in that the information could be first
/// retrieved from Celestia before Sequencer and executed against the rollup. In that case step 3.
/// would be skipped (no soft commitment update).
#[tokio::test]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 1,
        celestia_base_block_height: 1,
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

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_height: 3,
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
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test]
async fn pending_blocks_are_fetched() {
    let test_conductor = spawn_conductor(CommitLevel::SoftAndFirm).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_genesis_block_height: 1,
        celestia_base_block_height: 1,
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
    );

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_sequencer_genesis!(test_conductor);

    mount_pending_blocks!(test_conductor,
        [(
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        )],
    );

    // mount the Celestia blob containing the pending block to force the confirmation,
    // but don't mount the filtered sequencer block request yet

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_height: 3,
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

    // mount_get_filtered_sequencer_block!(
    //     test_conductor,
    //     sequencer_height: 2,
    // );

    // let execute_block = mount_executed_block!(
    //     test_conductor,
    //     number: 2,
    //     hash: [2; 64],
    //     parent: [1; 64],
    // );

    // let update_commitment_state_soft = mount_update_commitment_state!(
    //     test_conductor,
    //     firm: (
    //         number: 1,
    //         hash: [1; 64],
    //         parent: [0; 64],
    //     ),
    //     soft: (
    //         number: 2,
    //         hash: [2; 64],
    //         parent: [1; 64],
    //     ),
    // );

    // let update_commitment_state_firm = mount_update_commitment_state!(
    //     test_conductor,
    //     firm: (
    //         number: 2,
    //         hash: [2; 64],
    //         parent: [1; 64],
    //     ),
    //     soft: (
    //         number: 2,
    //         hash: [2; 64],
    //         parent: [1; 64],
    //     ),
    // );

    // timeout(
    //     Duration::from_millis(1000),
    //     join3(
    //         execute_block.wait_until_satisfied(),
    //         update_commitment_state_soft.wait_until_satisfied(),
    //         update_commitment_state_firm.wait_until_satisfied(),
    //     ),
    // )
    // .await
    // .expect(
    //     "conductor should have executed the soft block and updated the soft commitment state \
    //      within 1000ms",
    // );
}
