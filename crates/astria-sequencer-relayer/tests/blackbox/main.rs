#![allow(clippy::missing_panics_doc)]

pub mod helper;

use std::{
    collections::HashSet,
    time::Duration,
};

use assert_json_diff::assert_json_include;
use astria_core::{
    primitive::v1::RollupId,
    protocol::test_utils::ConfigureCometBftBlock,
};
use helper::{
    CometBftBlockToMount,
    TestSequencerRelayerConfig,
};
use reqwest::StatusCode;
use serde_json::json;
use tokio::time::{
    sleep,
    timeout,
};

const RELAY_SELF: bool = true;
const RELAY_ALL: bool = false;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn report_degraded_if_block_fetch_fails() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        rollup_id_filter: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    // Relayer reports 200 on /readyz after start
    let wait_for_readyz = async {
        loop {
            let readyz = reqwest::get(format!("http://{}/readyz", sequencer_relayer.api_address))
                .await
                .unwrap();
            if readyz.status().is_success() {
                break readyz;
            }
            sleep(Duration::from_millis(100)).await;
        }
    };
    let readyz = timeout(Duration::from_secs(1), wait_for_readyz)
        .await
        .expect("sequencer must report ready for test to work");

    assert_eq!(
        StatusCode::OK,
        readyz.status(),
        "relayer should report 200 after start"
    );
    assert_json_include!(
        expected: json!({"status": "ok"}),
        actual: readyz.json::<serde_json::Value>().await.unwrap(),
    );

    // mount a bad block next, so the relayer will fail to fetch the block
    let abci_guard = sequencer_relayer.mount_abci_response(1).await;
    let block_guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::BadAtHeight(1));
    timeout(
        Duration::from_millis(2 * sequencer_relayer.config.block_time),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occurred")
    .1
    .unwrap();

    // Relayer reports 500 on /healthz after fetching the block failed
    let readyz = reqwest::get(format!("http://{}/healthz", sequencer_relayer.api_address))
        .await
        .unwrap();

    assert_eq!(
        StatusCode::INTERNAL_SERVER_ERROR,
        readyz.status(),
        "relayer should report 500 when failing to fetch block"
    );
    assert_json_include!(
        expected: json!({"status": "degraded"}),
        actual: readyz.json::<serde_json::Value>().await.unwrap(),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn one_block_is_relayed_to_celestia() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        rollup_id_filter: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let abci_guard = sequencer_relayer.mount_abci_response(1).await;
    let block_guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(1));
    timeout(
        Duration::from_millis(100),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occurred")
    .1
    .unwrap();

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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn later_height_in_state_leads_to_expected_relay() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: Some(5),
        rollup_id_filter: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let abci_guard = sequencer_relayer.mount_abci_response(7).await;
    let block_guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(6));
    timeout(
        Duration::from_millis(100),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occurred")
    .1
    .unwrap();

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

    // XXX: Waiting in async tests is generally. Fix this by providing different means of
    // of orchestrating this test, for example by updating the sequencer relayer status API,
    // or observing graceful shutdown.
    tokio::time::sleep(Duration::from_secs(1)).await;
    sequencer_relayer.assert_state_files_are_as_expected(6, 6);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn three_blocks_are_relayed() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        rollup_id_filter: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(1));

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(2));

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(3));

    let expected_number_of_blobs = 6;
    let block_time = sequencer_relayer.config.block_time;

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
        Duration::from_millis(block_time * 4),
        observe_blobs,
    )
    .await
    .expect("blobs should be received after waiting for twice the sequencer block time");

    assert_eq!(
        expected_number_of_blobs, blobs_seen,
        "expected 6 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn block_from_other_proposer_is_skipped() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: true,
        last_written_sequencer_height: None,
        rollup_id_filter: HashSet::new(),
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_SELF>(CometBftBlockToMount::GoodAtHeight(1));

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::GoodAtHeight(2));

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let _guard =
        sequencer_relayer.mount_block_response::<RELAY_SELF>(CometBftBlockToMount::GoodAtHeight(3));

    let expected_number_of_blobs = 4;
    let block_time = sequencer_relayer.config.block_time;

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
        Duration::from_millis(block_time * 4),
        observe_blobs,
    )
    .await
    .expect("blobs should be received after waiting for four times the sequencer block time");

    assert_eq!(
        expected_number_of_blobs, blobs_seen,
        "expected 4 blobs in total, 1 header blob and 1 rollup blob per block"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn should_filter_rollup() {
    let included_rollup_ids: HashSet<_> = (0..5).map(|x| RollupId::new([x; 32])).collect();
    let excluded_rollup_ids: HashSet<_> = (0..5).map(|x| RollupId::new([100 + x; 32])).collect();

    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
        rollup_id_filter: included_rollup_ids.clone(),
    }
    .spawn_relayer()
    .await;

    // Create one transaction per rollup ID.
    let rollup_transactions = included_rollup_ids
        .iter()
        .chain(excluded_rollup_ids.iter())
        .map(|id| (*id, vec![1; 1]))
        .collect();

    let block = ConfigureCometBftBlock {
        height: 1,
        rollup_transactions,
        ..Default::default()
    }
    .make();

    let abci_guard = sequencer_relayer.mount_abci_response(1).await;
    let block_guard =
        sequencer_relayer.mount_block_response::<RELAY_ALL>(CometBftBlockToMount::Block(block));
    timeout(
        Duration::from_millis(100),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occurred")
    .1
    .unwrap();

    let Some(blobs_seen_by_celestia) = sequencer_relayer
        .celestia
        .state_rpc_confirmed_rx
        .recv()
        .await
    else {
        panic!("celestia must have seen blobs")
    };

    // There should be one blob per included rollup ID + one blob for sequencer namespace data.
    assert_eq!(blobs_seen_by_celestia.len(), included_rollup_ids.len() + 1);

    let seen_namespaces: HashSet<_> = blobs_seen_by_celestia
        .iter()
        .map(|blob| blob.namespace)
        .collect();

    // Check all included rollups IDs are actually included in the seen blobs.
    for included_rollup_id in included_rollup_ids {
        let namespace = celestia_client::celestia_namespace_v0_from_rollup_id(included_rollup_id);
        assert!(seen_namespaces.contains(&namespace));
    }

    // Check all excluded rollups IDs are actually excluded from the seen blobs.
    for excluded_rollup_id in excluded_rollup_ids {
        let namespace = celestia_client::celestia_namespace_v0_from_rollup_id(excluded_rollup_id);
        assert!(!seen_namespaces.contains(&namespace));
    }
}
