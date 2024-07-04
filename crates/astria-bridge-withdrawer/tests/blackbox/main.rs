use std::time::Duration;

use helpers::TestBridgeWithdrawer;

pub mod helpers;

#[tokio::test]
async fn startup_success() {
    let bridge_withdrawer = TestBridgeWithdrawer::spawn().await;

    bridge_withdrawer
        .timeout_ms(
            1000,
            "startup",
            tokio::time::sleep(Duration::from_millis(2000)),
        )
        .await;
}

#[tokio::test]
async fn watch_and_submit_sanity_check() {
    // let bridge_withdrawer = BridgeWithdrawer::spawn().await;
    // mount expected tx received from submitter on sequencer
    // push event thru anvil
}
