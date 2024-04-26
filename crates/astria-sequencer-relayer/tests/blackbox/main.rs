#![allow(clippy::missing_panics_doc)]

pub mod helpers;

use std::collections::HashSet;

use astria_core::{
    primitive::v1::RollupId,
    protocol::test_utils::ConfigureSequencerBlock,
};
use helpers::{
    SequencerBlockToMount,
    TestSequencerRelayerConfig,
};
use reqwest::StatusCode;
use tendermint::account::Id as AccountId;

const RELAY_SELF: bool = true;
const RELAY_ALL: bool = false;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn one_block_is_relayed_to_celestia() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(1);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    // The `MIN_POLL_INTERVAL_SECS` is 1, meaning the relayer waits for 1 second before attempting
    // the first `GetTx`, so we wait for 2 seconds.
    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // Assert the relayer reports the correct Celestia and sequencer heights.
    sequencer_relayer
        .wait_for_latest_confirmed_celestia_height(53, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_fetched_sequencer_height(1, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_observed_sequencer_height(1, 1_000)
        .await;

    assert_eq!(
        sequencer_relayer.celestia_app_received_blob_count(),
        2,
        "expected 2 blobs in total, 1 header blob and 1 rollup blob"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn report_degraded_if_block_fetch_fails() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    // Relayer reports 200 on /readyz after start.
    let readyz_status = sequencer_relayer
        .wait_for_readyz(StatusCode::OK, 1_000, "waiting for readyz")
        .await;
    assert_eq!(readyz_status, "ok");

    // Mount a good block, so the relayer will report 200 on /healthz.
    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(1);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    let healthz_status = sequencer_relayer
        .wait_for_healthz(StatusCode::OK, 2_000, "waiting for first healthz")
        .await;
    assert_eq!(healthz_status, "ok");
    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // Mount a bad block next, so the relayer will fail to fetch the block.
    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let block_to_mount = SequencerBlockToMount::BadAtHeight(2);
    let block_guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "bad block 2")
        .await;

    // Relayer reports 500 on /healthz after fetching the block failed.
    let healthz_status = sequencer_relayer
        .wait_for_healthz(
            StatusCode::INTERNAL_SERVER_ERROR,
            2_000,
            "waiting for second healthz",
        )
        .await;
    assert_eq!(healthz_status, "degraded");

    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for sequencer block guard",
            block_guard.wait_until_satisfied(),
        )
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn later_height_in_state_leads_to_expected_relay() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: Some(5),
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(6).await;
    let _guard = sequencer_relayer.mount_abci_response(7).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(6);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // Assert the relayer reports the correct Celestia and sequencer heights.
    sequencer_relayer
        .wait_for_latest_confirmed_celestia_height(53, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_fetched_sequencer_height(6, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_observed_sequencer_height(7, 1_000)
        .await;

    assert_eq!(
        sequencer_relayer.celestia_app_received_blob_count(),
        2,
        "expected 2 blobs in total, 1 header blob and 1 rollup blob"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn three_blocks_are_relayed() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(1);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(2);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 2")
        .await;

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(3);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 3")
        .await;

    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 2")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 2")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 3")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 3")
        .await;
    // Each block will have taken ~1 second due to the delay before each `GetTx`, so use 4.5
    // seconds.
    sequencer_relayer
        .timeout_ms(
            4_500,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // Assert the relayer reports the correct Celestia and sequencer heights.
    sequencer_relayer
        .wait_for_latest_confirmed_celestia_height(53, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_fetched_sequencer_height(3, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_observed_sequencer_height(3, 1_000)
        .await;

    assert_eq!(
        sequencer_relayer.celestia_app_received_blob_count(),
        6,
        "expected 6 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn block_from_other_proposer_is_skipped() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: true,
        last_written_sequencer_height: None,
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(1);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_SELF>(block_to_mount, "good block 1")
        .await;

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(2);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 2")
        .await;

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(3);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_SELF>(block_to_mount, "good block 3")
        .await;

    // We only expect two broadcast/get Tx gRPCs - block 2 should not have been broadcast.
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 2")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 2")
        .await;
    // Each block will have taken ~1 second due to the delay before each `GetTx`, so use 4 seconds.
    sequencer_relayer
        .timeout_ms(
            4_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // Assert the relayer reports the correct Celestia and sequencer heights.
    sequencer_relayer
        .wait_for_latest_confirmed_celestia_height(53, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_fetched_sequencer_height(3, 1_000)
        .await;
    sequencer_relayer
        .wait_for_latest_observed_sequencer_height(3, 1_000)
        .await;

    assert_eq!(
        sequencer_relayer.celestia_app_received_blob_count(),
        4,
        "expected 4 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn should_filter_rollup() {
    let included_rollup_ids: HashSet<_> = (0..5).map(|x| RollupId::new([x; 32])).collect();
    let excluded_rollup_ids: HashSet<_> = (0..5).map(|x| RollupId::new([100 + x; 32])).collect();

    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        only_include_rollups: included_rollup_ids.clone(),
    }
    .spawn_relayer()
    .await;

    // Create one transaction per rollup ID.
    let sequence_data = included_rollup_ids
        .iter()
        .chain(excluded_rollup_ids.iter())
        .map(|id| (*id, vec![1; 1]))
        .collect();
    let block = ConfigureSequencerBlock {
        block_hash: Some([99u8; 32]),
        height: 1,
        proposer_address: Some(AccountId::try_from(vec![0u8; 20]).unwrap()),
        sequence_data,
        ..Default::default()
    }
    .make();
    let block_to_mount = SequencerBlockToMount::Block(block);

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;
    let _guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    // There should be one blob per included rollup ID + one blob for sequencer namespace data.
    assert_eq!(
        sequencer_relayer.celestia_app_received_blob_count(),
        included_rollup_ids.len() + 1
    );

    // Check all included rollups IDs are actually included in the seen blobs.
    for included_rollup_id in included_rollup_ids {
        assert!(sequencer_relayer.has_celestia_app_received_blob_from_rollup(included_rollup_id));
    }

    // Check all excluded rollups IDs are actually excluded from the seen blobs.
    for excluded_rollup_id in excluded_rollup_ids {
        assert!(!sequencer_relayer.has_celestia_app_received_blob_from_rollup(excluded_rollup_id));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn should_shut_down() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        only_include_rollups: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    // Start handling a block.
    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let block_to_mount = SequencerBlockToMount::GoodAtHeight(1);
    let _guard = sequencer_relayer
        .mount_sequencer_block_response::<RELAY_ALL>(block_to_mount, "good block 1")
        .await;
    let broadcast_guard = sequencer_relayer
        .mount_celestia_app_broadcast_tx_response("broadcast tx 1")
        .await;
    sequencer_relayer
        .timeout_ms(
            1_000,
            "waiting for broadcast guard",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    // Send the shutdown signal - equivalent to sigkill being issued to sequencer-relayer process.
    sequencer_relayer.relayer_shutdown_handle.take();

    let get_tx_guard = sequencer_relayer
        .mount_celestia_app_get_tx_response(53, "get tx 1")
        .await;
    sequencer_relayer
        .timeout_ms(
            2_000,
            "waiting for get tx guard",
            get_tx_guard.wait_until_satisfied(),
        )
        .await;

    sequencer_relayer.wait_for_relayer_shutdown(1_000).await;
}
