use std::time::Duration;

use astria_sequencer_relayer::api;
use tokio::time;

// mod data_availability;
mod helper;
use helper::{
    spawn_sequencer_relayer,
    TestSequencerRelayer,
};

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn test_init() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let TestSequencerRelayer {
        api_address,
        mut celestia,
        conductor,
        original_block_response,
        sequencer: _sequencer,
        sequencer_relayer: _sequencer_relayer,
        _keyfile,
        config,
    } = spawn_sequencer_relayer().await;

    let () = helper::loop_until_conductor_has_subscribed(api_address).await;

    // Advance the time so that sequencer relayer polls sequencer
    time::advance(Duration::from_millis(config.block_time + 100)).await;

    let block_received_by_conductor = conductor.block_rx.await.unwrap();
    assert_eq!(
        original_block_response.block.header.data_hash,
        block_received_by_conductor.header.data_hash,
    );

    let blobs_received_by_celestia = celestia.rpc_confirmed_rx.recv().await.unwrap();
    // We can reconstruct the individual blobs here, but let's just assert that it's
    // two blobs for now: one transaction in the original block + sequencer namespace
    // data.
    assert_eq!(blobs_received_by_celestia.len(), 2);

    let api::Status {
        current_sequencer_height,
        current_data_availability_height,
        ..
    } = helper::get_api_status(api_address).await;
    assert_eq!(Some(1), current_sequencer_height);
    assert_eq!(Some(100), current_data_availability_height);

    // sequencer_relayer.await.unwrap();

    // gracefully exit the inhibit task
    inhibit_tx.send(()).unwrap();
}
