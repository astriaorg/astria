pub mod helper;

use astria_sequencer_relayer::config::Config;
use helper::{
    spawn_sequencer_relayer,
    BlockResponseFourLinkChain,
    BlockResponseThreeLinkChain,
    BlockResponseTwoLinkChain,
    CelestiaMode,
};
use tokio::{
    sync::mpsc::error::TryRecvError,
    time::Duration,
};

#[tokio::test(start_paused = true)]
async fn one_block_is_relayed_to_celestia_and_conductor() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer =
        spawn_sequencer_relayer(Config::default(), CelestiaMode::Immediate).await;

    let BlockResponseTwoLinkChain {
        genesis,
        child,
    } = helper::mount_constant_block_response_child_parent_pair(&sequencer_relayer).await;

    for block in [&genesis, &child] {
        // advance the sequencer ticker once to poll the sequencer for once block. receiving child
        // finalizes parent.
        sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;

        let Some(block_seen_by_conductor) = sequencer_relayer.conductor.block_rx.recv().await
        else {
            panic!("conductor must have seen one block")
        };
        assert_eq!(
            block.block.header.hash(),
            block_seen_by_conductor.block_hash(),
        );
    }

    // finalized parent is submitted to data availability and seen by celestia
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

#[tokio::test(start_paused = true)]
async fn same_block_is_dropped() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer =
        spawn_sequencer_relayer(Config::default(), CelestiaMode::Immediate).await;

    let BlockResponseTwoLinkChain {
        genesis,
        child,
    } = helper::mount_constant_block_response_child_parent_pair(&sequencer_relayer).await;

    for block in [&genesis, &child] {
        // advance the sequencer ticker once to poll the sequencer for once block. receiving child
        // finalizes parent.
        sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;

        let Some(block_seen_by_conductor) = sequencer_relayer.conductor.block_rx.recv().await
        else {
            panic!("conductor must have seen one block")
        };
        assert_eq!(
            block.block.header.data_hash,
            block_seen_by_conductor.header().data_hash,
        );
    }

    // finalized parent is submitted to data availability and seen by celestia
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

    sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;
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

#[tokio::test(start_paused = true)]
async fn celestia_bundles_blobs() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let mut sequencer_relayer =
        spawn_sequencer_relayer(Config::default(), CelestiaMode::Immediate).await;
    let BlockResponseFourLinkChain {
        genesis,
        parent,
        child,
        grandchild,
    } = helper::mount_4_changing_block_responses(&sequencer_relayer).await;

    let tick = Duration::from_millis(sequencer_relayer.config.block_time);
    let test_start = tokio::time::Instant::now();

    // - grandparent is received at 1 tick
    // - parent is received at 4 ticks -> finalizes itself and grandparent (submission to da
    // bundles these two)
    // - child is received at 2 ticks
    // - grandchild is received at 3 ticks -> finalizes child

    // advance the sequencer ticker by 1 four times and observe that conductor sees all blocks
    // published to gossip net. although in this mock set up, parent, child and grandchild are all
    // ready to be received at 2 ticks, relayer only polls sequencer for one block per tick.
    for mounted_block in [genesis, child, grandchild, parent] {
        sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;
        let block_seen_by_conductor = sequencer_relayer.conductor.block_rx.recv().await.unwrap();
        assert_eq!(
            mounted_block.block.header.hash(),
            block_seen_by_conductor.block_hash()
        );
    }

    assert_eq!(test_start.elapsed(), 4 * tick);

    // child finalizes upon receiving grandchild from sequencer at 3 ticks. celestia sees a pair of
    // blobs (1 block + sequencer namespace data)
    let blobs_seen_by_celestia = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .try_recv()
        .unwrap();
    assert_eq!(2, blobs_seen_by_celestia.len());

    // grandparent and parent finalizes upon receiving parent from sequencer at 4 ticks. celestia
    // sees a pair of blobs (1 block + sequencer namespace data)
    let blobs_seen_by_celestia = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .try_recv()
        .unwrap();

    assert_eq!(4, blobs_seen_by_celestia.len());

    let blobs_seen_by_celestia = sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv();

    assert!(blobs_seen_by_celestia.is_err());

    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}

#[tokio::test(start_paused = true)]
async fn slow_celestia_leads_to_bundled_blobs() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    // Start the environment with celestia delaying responses by 5 times the configured sequencer
    // block time (it takes 5000 ms to respond if the sequencer block time is 1000 ms)
    const CELESTIA_DELAY_TICKS: u64 = 5;
    let config = Config::default();
    // sequencer interval tick
    let tick = Duration::from_millis(config.block_time);
    // the ticks at which celestia network will see blobs. this is when finalization happens, i.e.
    // when blocks will be submitted to da, + delay. finalization happens at tick 3 and tick 4.
    let first_blobs_tick: u64 = 3 + CELESTIA_DELAY_TICKS;
    let second_blobs_tick: u64 = 4 + CELESTIA_DELAY_TICKS;
    // 0 ticks
    let test_start = tokio::time::Instant::now();

    let mut sequencer_relayer = spawn_sequencer_relayer(
        config,
        CelestiaMode::DelayedSinceFinalization(CELESTIA_DELAY_TICKS),
    )
    .await;

    // - grandparent is received at 1 tick
    // - parent is received at 4 ticks -> finalizes itself and grandparent (submission to da
    // bundles these two)
    // - child is received at 2 ticks
    // - grandchild is received at 3 ticks -> finalizes child
    let BlockResponseFourLinkChain {
        genesis,
        parent,
        child,
        grandchild,
    } = helper::mount_4_changing_block_responses(&sequencer_relayer).await;

    // advance the sequencer ticker by 1 four times and observe that conductor sees all blocks
    // published to gossip net. although parent, child and grandchild are all ready to be received
    // at 2 ticks, relayer only polls sequencer for one block per tick. todo(emhane): remove
    // restriction.
    for mounted_block in [genesis, child, grandchild, parent] {
        sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;
        let block_seen_by_conductor = sequencer_relayer.conductor.block_rx.recv().await.unwrap();
        assert_eq!(
            mounted_block.block.header.hash(),
            block_seen_by_conductor.block_hash()
        );
    }

    assert_eq!(test_start.elapsed(), 4 * tick);

    // advance until first da submission
    sequencer_relayer
        .advance_time_by_n_sequencer_ticks(first_blobs_tick - 4)
        .await;
    sequencer_relayer
        .advance_to_time_mod_block_time_not_zero(10)
        .await;

    // grandparent finalizes upon receiving parent at 2 ticks. celestia sees a pair of blobs 5
    // ticks later (1 block + sequencer namespace data)
    let blobs_seen_by_celestia = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .try_recv()
        .unwrap();
    assert_eq!(2, blobs_seen_by_celestia.len());

    assert_eq!(
        test_start.elapsed(),
        first_blobs_tick as u32 * tick + Duration::from_millis(10)
    );

    sequencer_relayer
        .advance_time_by_n_sequencer_ticks(second_blobs_tick - first_blobs_tick)
        .await;
    sequencer_relayer
        .advance_to_time_mod_block_time_not_zero(10)
        .await;

    // grandparent and parent finalizes upon receiving parent from sequencer at 4 ticks. celestia
    // sees a pair of blobs (1 block + sequencer namespace data)
    let blobs_seen_by_celestia = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .try_recv()
        .unwrap();

    assert_eq!(4, blobs_seen_by_celestia.len());

    let blobs_seen_by_celestia = sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv();

    assert!(blobs_seen_by_celestia.is_err());

    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_finalization_competing_blocks() {
    // TODO: Hack to inhibit tokio auto-advance in tests;
    // Replace once a follow-up to https://github.com/tokio-rs/tokio/pull/5200 lands
    let (inhibit_tx, inhibit_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || inhibit_rx.blocking_recv());

    let config = Config::default();
    // sequencer is polled for one block response every sequencer block time
    // todo(emhane): set constant queue time and default block time for test specifically,
    // shorten total test time
    let tick = Duration::from_millis(config.block_time);
    // 0 ticks
    let test_start = tokio::time::Instant::now();

    let mut sequencer_relayer = spawn_sequencer_relayer(config, CelestiaMode::Immediate).await;

    // - genesis is received at 1 tick
    // - parent one is received at 2 tick
    // - parent two is received at 3 ticks
    // - child one.one is received at 4 ticks -> finalizes genesis, makes parent one canonical,
    //   discards parent two as by ssf
    // - child one.two is received at 5 ticks
    // - grandchild one.one.two is received at 6 ticks -> finalizes parent one, makes child one.two
    //   canonical, discards child one.one as by ssf

    let (
        BlockResponseThreeLinkChain {
            genesis,
            parent: parent_one,
            child: child_one_one,
        },
        BlockResponseFourLinkChain {
            genesis: _,
            parent: parent_two,
            child: child_one_two,
            grandchild: grandchild_one_two_one,
        },
    ) = helper::mount_genesis_and_two_response_pairs(&sequencer_relayer).await;

    let genesis_block_hash = genesis.block_id.hash;
    let parent_one_block_hash = parent_one.block_id.hash;
    let parent_two_block_hash = parent_two.block_id.hash;
    let child_one_one_block_hash = child_one_one.block_id.hash;
    let child_one_two_block_hash = child_one_two.block_id.hash;

    for mounted_block in [
        genesis,
        parent_one,
        parent_two,
        child_one_one,
        child_one_two,
        grandchild_one_two_one,
    ] {
        let mounted_block_hash = mounted_block.block.header.hash();
        // advance time to poll sequencer for next block and submit it to gossip-net
        sequencer_relayer.advance_time_by_n_sequencer_ticks(1).await;

        assert_eq!(
            test_start.elapsed(),
            if mounted_block_hash == genesis_block_hash {
                // genesis is received at 1 tick
                tick
            } else if mounted_block_hash == parent_one_block_hash {
                // parent one is received at 2 ticks
                2 * tick
            } else if mounted_block_hash == parent_two_block_hash {
                // parent two is received at 3 ticks
                3 * tick
            } else if mounted_block_hash == child_one_one_block_hash {
                // child one is received at 4 ticks
                4 * tick
            } else if mounted_block_hash == child_one_two_block_hash {
                // child one is received at 5 ticks
                5 * tick
            } else {
                // child two is received at 6 ticks
                6 * tick
            }
        );

        // block submitted on gossip-net should be seen by conductor
        let block_seen_by_conductor = sequencer_relayer.conductor.block_rx.recv().await.unwrap();

        assert_eq!(mounted_block_hash, block_seen_by_conductor.block_hash());

        if test_start.elapsed() == 4 * tick {
            // genesis finalizes at height 4. celestia sees a pair of blobs (1 block + sequencer
            // namespace data).
            let blobs_seen_by_celestia = sequencer_relayer
                .celestia
                .state_rpc_confirmed_rx
                .try_recv()
                .unwrap();

            assert_eq!(2, blobs_seen_by_celestia.len());
        } else if test_start.elapsed() == 6 * tick {
            // parent one finalizes at height 4. celestia sees a pair of blobs (1 block + sequencer
            // namespace data).
            let blobs_seen_by_celestia = sequencer_relayer
                .celestia
                .state_rpc_confirmed_rx
                .try_recv()
                .unwrap();

            assert_eq!(2, blobs_seen_by_celestia.len());
        }
    }

    sequencer_relayer
        .advance_to_time_mod_block_time_not_zero(10)
        .await;

    // parent two is discarded
    let blobs_seen_by_celestia = sequencer_relayer.celestia.state_rpc_confirmed_rx.try_recv();

    assert!(blobs_seen_by_celestia.is_err());

    // TODO: we should shut down and join all outstanding tasks here.

    // gracefully exit the inhibited task
    inhibit_tx.send(()).unwrap();
}
