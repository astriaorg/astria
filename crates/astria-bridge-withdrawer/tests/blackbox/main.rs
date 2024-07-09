use helpers::{
    astria_address,
    TestBridgeWithdrawer,
};

pub mod helpers;

#[tokio::test]
async fn sequencer_withdraw_success() {
    let bridge_withdrawer = TestBridgeWithdrawer::spawn().await;

    let nonce_guard = bridge_withdrawer
        .mount_pending_nonce_response_as_scoped(1, "process batch 1")
        .await;
    let submission_guard = bridge_withdrawer
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // send a tx to the rollup
    let value = 1_000_000.into();
    let recipient = astria_address([1u8; 20]);
    bridge_withdrawer
        .ethereum
        .send_sequencer_withdraw_transaction(value, recipient)
        .await;

    bridge_withdrawer
        .timeout_ms(100, "startup", nonce_guard.wait_until_satisfied())
        .await;
    bridge_withdrawer
        .timeout_ms(100, "startup", submission_guard.wait_until_satisfied())
        .await;
}
