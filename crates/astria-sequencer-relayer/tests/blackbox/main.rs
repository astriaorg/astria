pub mod helper;

use std::time::Duration;

use helper::{
    spawn_sequencer_relayer,
    CelestiaMode,
};
use tokio::{
    sync::mpsc::error::TryRecvError,
    time::{
        self,
        timeout,
    },
};

#[tokio::test(flavor = "current_thread")]
async fn one_block_is_relayed_to_celestia() {
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;
    'first_latest_block: {
        let guard = sequencer_relayer.mount_block_response(1).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'first_latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

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
async fn same_block_is_dropped() {
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;
    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(1).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

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
    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(1).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

    match sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        other => panic!("celestia should have not seen a blob, but returned {other:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn slow_celestia_leads_to_bundled_blobs() {
    // Start the environment with celestia delaying responses by 4 times the sequencer block time
    // (it takes 4000 ms to respond if the sequencer block time is 1000 ms).
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Delayed(4)).await;

    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(1).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(2).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(3).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

    'latest_block: {
        let guard = sequencer_relayer.mount_block_response(4).await;
        if timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .is_ok()
        {
            break 'latest_block;
        }
        time::pause();
        sequencer_relayer.advance_by_block_time().await;
        time::resume();
        timeout(Duration::from_millis(100), guard.wait_until_satisfied())
            .await
            .unwrap();
    }

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
}
