use astria_bridge_withdrawer::BridgeWithdrawer;

mod helpers;

#[tokio::test]
async fn startup_success() {
    let _bridge_withdrawer = BridgeWithdrawer::spawn().await;
}

#[tokio::test]
async fn watch_and_submit_sanity_check() {
    // let bridge_withdrawer = BridgeWithdrawer::spawn().await;
    // mount expected tx received from submitter on sequencer
    // push event thru anvil
}
