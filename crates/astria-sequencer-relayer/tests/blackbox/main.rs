#![allow(clippy::missing_panics_doc)]

pub mod helper;

use std::time::Duration;

use helper::spawn_sequencer_relayer;
use tokio::time::timeout;

const RELAY_SELF: bool = true;
const RELAY_ALL: bool = false;

#[tokio::test(flavor = "current_thread")]
async fn one_block_is_relayed_to_celestia() {
    let mut sequencer_relayer = spawn_sequencer_relayer::<RELAY_ALL>().await;

    let abci_guard = sequencer_relayer.mount_abci_response(1).await;
    let block_guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(1).await;
    timeout(
        Duration::from_millis(100),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occured");

    let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    else {
        panic!("celestia must have seen blobs")
    };
    // We can reconstruct the individual blobs here, but let's just assert that it's
    // two blobs for now: one transaction in the original block + sequencer namespace
    // data.
    assert_eq!(blobs_seen_by_celestia.len(), 2);

    // TODO: we should shut down and join all outstanding tasks here.
}

#[tokio::test(flavor = "current_thread")]
async fn three_blocks_are_relayed() {
    let mut sequencer_relayer = spawn_sequencer_relayer::<RELAY_ALL>().await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(1).await;

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(2).await;

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(3).await;

    let expected_number_of_blobs = 6;

    let observe_blobs = async move {
        let mut blobs_seen = 0;
        while let Some(blobs) = sequencer_relayer
            .celestia
            .state_rpc_confirmed_rx
            .recv()
            .await
        {
            blobs_seen += blobs.len();
            if blobs_seen >= expected_number_of_blobs {
                break;
            }
        }
        blobs_seen
    };

    let blobs_seen = timeout(
        // timeout after (3 + 1) block times to ensure that 3 blocks are definitely picked up
        Duration::from_millis(sequencer_relayer.config.block_time * 4),
        observe_blobs,
    )
    .await
    .expect("blobs should be received after waiting for twice the sequencer block time");

    assert_eq!(
        expected_number_of_blobs, blobs_seen,
        "expected 6 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn block_from_other_proposer_is_skipped() {
    let mut sequencer_relayer = spawn_sequencer_relayer::<RELAY_SELF>().await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard = sequencer_relayer
        .mount_block_response::<RELAY_SELF>(1)
        .await;

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(2).await;

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let _guard = sequencer_relayer
        .mount_block_response::<RELAY_SELF>(3)
        .await;

    let expected_number_of_blobs = 4;

    let observe_blobs = async move {
        let mut blobs_seen = 0;
        while let Some(blobs) = sequencer_relayer
            .celestia
            .state_rpc_confirmed_rx
            .recv()
            .await
        {
            blobs_seen += blobs.len();
            if blobs_seen >= expected_number_of_blobs {
                break;
            }
        }
        blobs_seen
    };

    let blobs_seen = timeout(
        // timeout after (3 + 1) block times to ensure that 3 blocks are definitely picked up
        Duration::from_millis(sequencer_relayer.config.block_time * 4),
        observe_blobs,
    )
    .await
    .expect("blobs should be received after waiting for four times the sequencer block time");

    assert_eq!(
        expected_number_of_blobs, blobs_seen,
        "expected 4 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}
