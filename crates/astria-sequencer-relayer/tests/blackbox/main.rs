pub mod helper;

use helper::{
    spawn_sequencer_relayer,
    CelestiaMode,
};
use tokio::sync::mpsc::error::TryRecvError;

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn one_block_is_relayed_to_celestia() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;
    let _ = helper::mount_constant_block_response(&sequencer_relayer).await;

    // Advance by the configured sequencer block time to get one block
    // from the sequencer.
    sequencer_relayer.advance_by_block_time().await;

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

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn same_block_is_dropped() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;
    let _ = helper::mount_constant_block_response(&sequencer_relayer).await;

    // Advance by the configured sequencer block time to get one block
    // from the sequencer.
    sequencer_relayer.advance_by_block_time().await;

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
    sequencer_relayer.advance_by_block_time().await;

    match sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        other => panic!("celestia should have not seen a blob, but returned {other:?}"),
    }

    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn slow_celestia_leads_to_bundled_blobs() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    // Start the environment with celestia delaying responses by 5 times the sequencer block time
    // (it takes 5000 ms to respond if the sequencer block time is 1000 ms).
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Delayed(5)).await;
    let all_blocks = helper::mount_4_changing_block_responses(&sequencer_relayer).await;

    // Advance the block 8 times and observe that conductor sees all events immediately
    for _ in all_blocks.iter().take(4) {
        sequencer_relayer.advance_by_block_time().await;
    }

    // Advance twice more to trigger the celestia response.
    sequencer_relayer.advance_by_block_time().await;

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

    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}
