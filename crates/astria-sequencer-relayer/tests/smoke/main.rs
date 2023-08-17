pub mod helper;

use helper::{
    spawn_sequencer_relayer,
    CelestiaMode,
};
use tokio::sync::mpsc::error::TryRecvError;

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn one_block_is_relayed_to_celestia_and_conductor() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;
    let expected_block_response = helper::mount_constant_block_response(&sequencer_relayer).await;

    // Advance by the configured sequencer block time to get one block
    // from the sequencer.
    sequencer_relayer.advance_by_block_time().await;

    let Some(block_seen_by_conductor) = sequencer_relayer.conductor.block_rx.recv().await else {
        panic!("conductor must have seen one block")
    };
    assert_eq!(
        expected_block_response.block.header.data_hash,
        block_seen_by_conductor.header.data_hash,
    );

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
    let expected_block_response = helper::mount_constant_block_response(&sequencer_relayer).await;

    // Advance by the configured sequencer block time to get one block
    // from the sequencer.
    sequencer_relayer.advance_by_block_time().await;

    let Some(block_seen_by_conductor) = sequencer_relayer.conductor.block_rx.recv().await else {
        panic!("conductor must have seen one block")
    };
    assert_eq!(
        expected_block_response.block.header.data_hash,
        block_seen_by_conductor.header.data_hash,
    );

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
    match sequencer_relayer.conductor.block_rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        other => panic!("conductor should have not seen a block, but returned {other:?}"),
    }
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
    let mut sequencer_relayer =
        spawn_sequencer_relayer(CelestiaMode::DelayedSinceResponse(5)).await;
    let all_blocks = helper::mount_4_changing_block_responses(&sequencer_relayer).await;

    // Advance the block 8 times and observe that conductor sees all events immediately
    for mounted_block in all_blocks.iter().take(4) {
        sequencer_relayer.advance_by_block_time().await;
        let block_seen_by_conductor = sequencer_relayer.conductor.block_rx.recv().await.unwrap();
        assert_eq!(
            mounted_block.block.header.data_hash,
            block_seen_by_conductor.header.data_hash,
        );
    }
    // Advancing the time one more will not be observed because the block response
    // is at the same height.
    sequencer_relayer.advance_by_block_time().await;
    let Err(TryRecvError::Empty) = sequencer_relayer.conductor.block_rx.try_recv() else {
        panic!("conductor observered another block although it shouldn't have");
    };

    // Advance once more to trigger the celestia response.
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

#[tokio::test(start_paused = true)]
async fn test_finalization() {
    use astria_sequencer_relayer::config::MAX_RELAYER_QUEUE_TIME_MS;
    use tokio::time::Duration;

    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let test_start = tokio::time::Instant::now();
    let mut sequencer_relayer = spawn_sequencer_relayer(CelestiaMode::Immediate).await;

    // children are delayed max queue time
    let (parent_one, parent_two, child_one, child_two) =
        helper::mount_2_parent_child_pair_block_responses(&sequencer_relayer).await;

    let parent_one_block_hash = helper::get_block_hash(&parent_one);
    let parent_two_block_hash = helper::get_block_hash(&parent_two);
    let child_one_block_hash = helper::get_block_hash(&child_one);

    for mounted_block in [parent_one, parent_two, child_one, child_two] {
        let mounted_block_hash = mounted_block.block.header.hash().as_bytes().to_vec();
        // advance time to poll sequencer for next block and submit it to gossip-net
        if mounted_block_hash == parent_one_block_hash
            || mounted_block_hash == parent_two_block_hash
        {
            // advance time once to receive parents from conductor
            sequencer_relayer.advance_by_block_time().await;
            assert_eq!(
                test_start.elapsed(),
                if mounted_block_hash == parent_one_block_hash {
                    // parent one is received at 1 sequencer block time + epsilon
                    Duration::from_millis(1010)
                } else {
                    // parent two is received at 2 sequencer block times + epsilon
                    Duration::from_millis(2020)
                }
            );
        } else {
            // advance time max relayer queue time for children, the time mock sequencer is set to
            // delay them (helper::mount_2_parent_children_pair_block_responses)
            let blocks = MAX_RELAYER_QUEUE_TIME_MS / sequencer_relayer.config.block_time_ms;
            // todo(emhane): set constant queue time and default block time for test specifically,
            // shorten total test time
            assert_eq!(blocks, 2);

            sequencer_relayer
                .advance_by_block_time_n_blocks(blocks)
                .await;
            assert_eq!(
                test_start.elapsed(),
                if mounted_block_hash == child_one_block_hash {
                    // receiving first child from sequencer, after max relayer queue time,
                    // finalizes parent one and times out parent two.
                    //
                    // child one is received at 4 sequencer block times + epsilon
                    Duration::from_millis(4040)
                } else {
                    // child two is received at 6 sequencer block times + epsilon
                    Duration::from_millis(6060)
                }
            );
        }

        // block submitted on gossip-net should be seen by conductor
        let block_seen_by_conductor = sequencer_relayer.conductor.block_rx.recv().await.unwrap();

        assert_eq!(mounted_block_hash, block_seen_by_conductor.block_hash);
    }

    // Advance once more to trigger the celestia response to 7 + epsilon block time
    sequencer_relayer.advance_by_block_time().await;
    assert_eq!(test_start.elapsed(), Duration::from_millis(7070));

    // only parent one finalizes. celestia sees a pair of blobs (1 block + sequencer namespace
    // data).
    let blobs_seen_by_celestia = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .try_recv()
        .unwrap();

    assert_eq!(2, blobs_seen_by_celestia.len());

    // parent two times out
    let blobs_seen_by_celestia = sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv();

    assert!(blobs_seen_by_celestia.is_err());
    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}
