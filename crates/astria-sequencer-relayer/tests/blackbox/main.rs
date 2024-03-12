#![allow(clippy::missing_panics_doc)]

pub mod helper;

use std::time::Duration;

use assert_json_diff::assert_json_include;
use helper::TestSequencerRelayerConfig;
use reqwest::StatusCode;
use serde_json::json;
use tokio::time::{
    sleep,
    timeout,
};

const RELAY_SELF: bool = true;
const RELAY_ALL: bool = false;

#[tokio::test(flavor = "current_thread")]
async fn report_degraded_if_block_fetch_fails() {
    let sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
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

    // don't mount another block, so the relayer will fail to fetch the block
    // note: this relies on relayer attempting to fetch a new block within `2 * block_time`,
    // if that doesn't happen, this test will fail.
    let abci_guard = sequencer_relayer.mount_abci_response(1).await;
    timeout(
        Duration::from_millis(2 * sequencer_relayer.config.block_time),
        abci_guard.wait_until_satisfied(),
    )
    .await
    .expect("requesting abci info and block must have occured");

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

    sequencer_relayer.shutdown();
}

#[tokio::test(flavor = "current_thread")]
async fn one_block_is_relayed_to_celestia() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
    }
    .spawn_relayer()
    .await;

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
    .expect("requesting abci info and block must have occured")
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

    sequencer_relayer.shutdown();
}

#[tokio::test(flavor = "current_thread")]
async fn later_height_in_state_leads_to_expected_relay() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: Some(5),
    }
    .spawn_relayer()
    .await;

    let abci_guard = sequencer_relayer.mount_abci_response(7).await;
    let block_guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(6).await;
    timeout(
        Duration::from_millis(100),
        futures::future::join(
            abci_guard.wait_until_satisfied(),
            block_guard.wait_until_satisfied(),
        ),
    )
    .await
    .expect("requesting abci info and block must have occured")
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

    sequencer_relayer.shutdown();
}

#[tokio::test(flavor = "current_thread")]
async fn three_blocks_are_relayed() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: false,
        last_written_sequencer_height: None,
    }
    .spawn_relayer()
    .await;

    let _guard = sequencer_relayer.mount_abci_response(1).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(1).await;

    let _guard = sequencer_relayer.mount_abci_response(2).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(2).await;

    let _guard = sequencer_relayer.mount_abci_response(3).await;
    let _guard = sequencer_relayer.mount_block_response::<RELAY_ALL>(3).await;

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
        sequencer_relayer.shutdown();
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

#[tokio::test(flavor = "current_thread")]
async fn block_from_other_proposer_is_skipped() {
    let mut sequencer_relayer = TestSequencerRelayerConfig {
        relay_only_self: true,
        last_written_sequencer_height: None,
    }
    .spawn_relayer()
    .await;

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
        sequencer_relayer.shutdown();
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
