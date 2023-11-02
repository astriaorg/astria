pub mod helper;

use std::time::Duration;

use helper::{
    spawn_sequencer_relayer,
    spawn_sequencer_relayer_relay_all,
    CelestiaMode,
    TestSequencerRelayer,
};
use tokio::{
    sync::mpsc::error::TryRecvError,
    time::{
        self,
        timeout,
    },
};
use wiremock::MockGuard;

/// Small hack because sometimes test choreography doesn't work properly, with
/// sequencer-relayer trying to get the latest block too early or too late.
/// This ensures that 1. the guard times out (so the test does not run indefinitely),
/// but that 2. sequencer-relayer's timer is triggered, pulling a new block from the mock.
async fn timeout_guard(test_env: &TestSequencerRelayer, guard: MockGuard) {
    if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
        .await
        .is_ok()
    {
        return;
    }
    time::pause();
    test_env.advance_by_block_time().await;
    time::resume();
    timeout(Duration::from_millis(100), guard.wait_until_satisfied())
        .await
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn one_block_is_relayed_to_celestia() {
    let mut sequencer_relayer = spawn_sequencer_relayer_relay_all(CelestiaMode::Immediate).await;
    let guard = sequencer_relayer.mount_block_response(1).await;
    timeout_guard(&sequencer_relayer, guard).await;

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
async fn one_block_is_relayed_to_celestia_relay_only_validator_key() {
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate, true).await;
    let guard = sequencer_relayer.mount_block_response(1).await;
    timeout_guard(&sequencer_relayer, guard).await;
    let guard = sequencer_relayer
        .mount_block_response_with_zero_proposer(1)
        .await;
    timeout_guard(&sequencer_relayer, guard).await;

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
async fn one_block_is_relayed_to_celestia_relay_all() {
    let mut sequencer_relayer = spawn_sequencer_relayer_relay_all(CelestiaMode::Immediate).await;
    let guard = sequencer_relayer
        .mount_block_response_with_zero_proposer(1)
        .await;
    timeout_guard(&sequencer_relayer, guard).await;

    let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    else {
        panic!("celestia must have seen blobs")
    };

    // we should have relayed the block even if it wasn't proposed by our address.
    assert_eq!(blobs_seen_by_celestia.len(), 2);

    // TODO: we should shut down and join all outstanding tasks here.
}

#[tokio::test(flavor = "current_thread")]
async fn same_block_is_dropped() {
    let mut sequencer_relayer = spawn_sequencer_relayer_relay_all(CelestiaMode::Immediate).await;
    let guard = sequencer_relayer.mount_block_response(1).await;
    timeout_guard(&sequencer_relayer, guard).await;

    // The first block should be received immediately
    let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    else {
        panic!("celestia must have seen blobs")
    };
    assert_eq!(blobs_seen_by_celestia.len(), 2);

    // Mount the same block again and advance by the block time to ensure its picked up.
    let guard = sequencer_relayer.mount_block_response(1).await;
    timeout_guard(&sequencer_relayer, guard).await;

    match sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        other => panic!("celestia should have not seen a blob, but returned {other:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn slow_celestia_leads_to_bundled_blobs() {
    // Start the environment with celestia delaying responses by 4 times the sequencer block time
    // (it takes 4000 ms to respond if the sequencer block time is 1000 ms).
    let mut sequencer_relayer = spawn_sequencer_relayer_relay_all(CelestiaMode::Delayed(4)).await;

    let guard = sequencer_relayer.mount_block_response(1).await;
    timeout_guard(&sequencer_relayer, guard).await;

    let guard = sequencer_relayer.mount_block_response(2).await;
    timeout_guard(&sequencer_relayer, guard).await;

    let guard = sequencer_relayer.mount_block_response(3).await;
    timeout_guard(&sequencer_relayer, guard).await;

    let guard = sequencer_relayer.mount_block_response(4).await;
    timeout_guard(&sequencer_relayer, guard).await;

    // But celestia sees a pair of blobs (1 block + sequencer namespace data)
    if let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    {
        assert_eq!(2, blobs_seen_by_celestia.len());
    }

    // And then all the remaining blobs arrive
    if let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    {
        assert_eq!(6, blobs_seen_by_celestia.len());
    }
}
