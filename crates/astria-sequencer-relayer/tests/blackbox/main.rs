#![allow(clippy::missing_panics_doc)]

pub mod helper;

use std::{
    collections::HashSet,
    time::Duration,
};

use assert_json_diff::assert_json_include;
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
