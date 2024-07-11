use helpers::{
    astria_address,
    make_bridge_unlock_action,
    signed_tx_from_request,
    TestBridgeWithdrawer,
};
use tendermint::v0_34::abci::request;

#[allow(clippy::missing_panics_doc)]
pub mod helpers;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn sequencer_withdraw_success() {
    let bridge_withdrawer = TestBridgeWithdrawer::spawn().await;

    let nonce_guard = bridge_withdrawer
        .mount_pending_nonce_response_as_scoped(1, "process batch 1")
        .await;
    let broadcast_guard = bridge_withdrawer
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // send a tx to the rollup
    let value = 1_000_000.into();
    let recipient = astria_address([1u8; 20]);
    let _receipt = bridge_withdrawer
        .ethereum
        .send_sequencer_withdraw_transaction(value, recipient)
        .await;

    bridge_withdrawer
        .timeout_ms(2_000, "batch 1 nonce", nonce_guard.wait_until_satisfied())
        .await;
    bridge_withdrawer
        .timeout_ms(
            2_000,
            "batch 1 execution",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    // check the submitted transaction
    let requests = broadcast_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    let tx = signed_tx_from_request(&requests[0]);
    let actions = tx.actions();
    assert_eq!(actions.len(), 1);

    let expected_action = make_bridge_unlock_action();
    let actual_action = actions[0].clone();
    assert!(matches!(expected_action, actual_action));
}
