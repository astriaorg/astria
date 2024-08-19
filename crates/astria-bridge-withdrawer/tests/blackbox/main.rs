use astria_core::protocol::transactions::v1alpha1::Action;
use helpers::{
    assert_actions_eq,
    default_sequencer_address,
    make_bridge_unlock_action,
    make_ics20_withdrawal_action,
    signed_tx_from_request,
    TestBridgeWithdrawerConfig,
};

pub mod helpers;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "needs anvil to be present in $PATH; see github.com/foundry-rs/foundry for how to \
            install"]
async fn native_sequencer_withdraw_success() {
    let test_env = TestBridgeWithdrawerConfig::default().spawn().await;

    test_env
        .mount_pending_nonce_response(1, "process batch 1")
        .await;
    let broadcast_guard = test_env
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // send a native sequencer withdrawal tx to the rollup
    let value = 1_000_000.into();
    let recipient = default_sequencer_address();
    let receipt = test_env
        .ethereum
        .send_sequencer_withdraw_transaction(value, recipient)
        .await;

    test_env
        .timeout_ms(
            2_000,
            "batch 1 execution",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    assert_contract_receipt_action_matches_broadcast_action::<BridgeUnlock>(
        &broadcast_guard.received_requests().await,
        &receipt,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "needs anvil to be present in $PATH; see github.com/foundry-rs/foundry for how to \
            install"]
async fn native_ics20_withdraw_success() {
    let test_env = TestBridgeWithdrawerConfig::native_ics20_config()
        .spawn()
        .await;

    test_env
        .mount_pending_nonce_response(1, "process batch 1")
        .await;
    let broadcast_guard = test_env
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // send an ics20 withdrawal tx to the rollup
    let value = 1_000_000.into();
    let recipient = default_sequencer_address();
    let receipt = test_env
        .ethereum
        .send_ics20_withdraw_transaction(value, recipient.to_string())
        .await;

    test_env
        .timeout_ms(
            2_000,
            "batch 1 execution",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    assert_contract_receipt_action_matches_broadcast_action::<Ics20>(
        &broadcast_guard.received_requests().await,
        &receipt,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "needs anvil to be present in $PATH; see gith&ub.com/foundry-rs/foundry for how to \
            install"]
async fn erc20_sequencer_withdraw_success() {
    let test_env = TestBridgeWithdrawerConfig::erc20_sequencer_withdraw_config()
        .spawn()
        .await;

    test_env
        .mount_pending_nonce_response(1, "process batch 1")
        .await;
    let broadcast_guard = test_env
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // mint some erc20 tokens to the rollup wallet
    let _mint_receipt = test_env
        .ethereum
        .mint_tokens(2_000_000_000.into(), test_env.rollup_wallet_addr())
        .await;

    // send an ics20 withdrawal tx to the rollup
    let value = 1_000_000.into();
    let recipient = default_sequencer_address();
    let receipt = test_env
        .ethereum
        .send_sequencer_withdraw_transaction_erc20(value, recipient)
        .await;

    test_env
        .timeout_ms(
            2_000,
            "batch 1 execution",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    assert_contract_receipt_action_matches_broadcast_action::<BridgeUnlock>(
        &broadcast_guard.received_requests().await,
        &receipt,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "needs anvil to be present in $PATH; see github.com/foundry-rs/foundry for how to \
            install"]
async fn erc20_ics20_withdraw_success() {
    let test_env = TestBridgeWithdrawerConfig::erc20_ics20_config()
        .spawn()
        .await;

    test_env
        .mount_pending_nonce_response(1, "process batch 1")
        .await;
    let broadcast_guard = test_env
        .mount_broadcast_tx_commit_success_response_as_scoped()
        .await;

    // mint some erc20 tokens to the rollup wallet
    let _mint_receipt = test_env
        .ethereum
        .mint_tokens(2_000_000_000.into(), test_env.rollup_wallet_addr())
        .await;

    // send an ics20 withdrawal tx to the rollup
    let value = 1_000_000.into();
    let recipient = default_sequencer_address();
    let receipt = test_env
        .ethereum
        .send_ics20_withdraw_transaction_astria_bridgeable_erc20(value, recipient.to_string())
        .await;

    test_env
        .timeout_ms(
            2_000,
            "batch 1 execution",
            broadcast_guard.wait_until_satisfied(),
        )
        .await;

    assert_contract_receipt_action_matches_broadcast_action::<Ics20>(
        &broadcast_guard.received_requests().await,
        &receipt,
    );
}

trait ActionFromReceipt {
    fn action_from_receipt(receipt: &ethers::types::TransactionReceipt) -> Action;
}

struct BridgeUnlock;
impl ActionFromReceipt for BridgeUnlock {
    #[track_caller]
    fn action_from_receipt(receipt: &ethers::types::TransactionReceipt) -> Action {
        make_bridge_unlock_action(receipt)
    }
}

struct Ics20;
impl ActionFromReceipt for Ics20 {
    #[track_caller]
    fn action_from_receipt(receipt: &ethers::types::TransactionReceipt) -> Action {
        make_ics20_withdrawal_action(receipt)
    }
}

#[track_caller]
fn assert_contract_receipt_action_matches_broadcast_action<T: ActionFromReceipt>(
    received_broadcasts: &[wiremock::Request],
    receipt: &ethers::types::TransactionReceipt,
) {
    let tx = signed_tx_from_request(received_broadcasts.first().expect(
        "at least one request should have been received if the broadcast guard is satisfied",
    ));
    let actual = tx
        .actions()
        .first()
        .expect("the signed transaction should contain at least one action");

    let expected = T::action_from_receipt(receipt);
    assert_actions_eq(&expected, actual);
}
