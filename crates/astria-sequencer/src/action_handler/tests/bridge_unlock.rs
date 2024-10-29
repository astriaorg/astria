use astria_core::{
    primitive::v1::{
        RollupId,
        TransactionId,
    },
    protocol::{
        fees::v1::BridgeUnlockFeeComponents,
        transaction::v1::action::BridgeUnlock,
    },
};
use cnidarium::StateDelta;

use crate::{
    accounts::StateWriteExt as _,
    action_handler::{
        tests::test_asset,
        ActionHandler as _,
    },
    address::StateWriteExt as _,
    benchmark_and_test_utils::{
        assert_eyre_error,
        astria_address,
        ASTRIA_PREFIX,
    },
    bridge::StateWriteExt as _,
    fees::StateWriteExt as _,
    transaction::{
        StateWriteExt as _,
        TransactionContext,
    },
};

#[tokio::test]
async fn bridge_unlock_fails_if_bridge_account_has_no_withdrawer_address() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_transaction_context(TransactionContext {
        address_bytes: [1; 20],
        transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    });
    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let asset = test_asset();
    let transfer_amount = 100;

    let to_address = astria_address(&[2; 20]);
    let bridge_address = astria_address(&[3; 20]);
    state
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();

    let bridge_unlock = BridgeUnlock {
        to: to_address,
        amount: transfer_amount,
        fee_asset: asset.clone(),
        memo: String::new(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    };

    // invalid sender, doesn't match action's `from`, should fail
    assert_eyre_error(
        &bridge_unlock.check_and_execute(state).await.unwrap_err(),
        "bridge account does not have an associated withdrawer address",
    );
}

#[tokio::test]
async fn bridge_unlock_fails_if_withdrawer_is_not_signer() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_transaction_context(TransactionContext {
        address_bytes: [1; 20],
        transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    });
    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let asset = test_asset();
    let transfer_amount = 100;

    let to_address = astria_address(&[2; 20]);
    let bridge_address = astria_address(&[3; 20]);
    let withdrawer_address = astria_address(&[4; 20]);
    state
        .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
        .unwrap();
    state
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();

    let bridge_unlock = BridgeUnlock {
        to: to_address,
        amount: transfer_amount,
        fee_asset: asset,
        memo: String::new(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    };

    // invalid sender, doesn't match action's bridge account's withdrawer, should fail
    assert_eyre_error(
        &bridge_unlock.check_and_execute(state).await.unwrap_err(),
        "unauthorized to unlock bridge account",
    );
}

#[tokio::test]
async fn bridge_unlock_executes_with_duplicated_withdrawal_event_id() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    let bridge_address = astria_address(&[1; 20]);
    state.put_transaction_context(TransactionContext {
        address_bytes: bridge_address.bytes(),
        transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    });
    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let asset = test_asset();
    let transfer_fee = 10;
    let transfer_amount = 100;
    state
        .put_bridge_unlock_fees(BridgeUnlockFeeComponents {
            base: transfer_fee,
            multiplier: 0,
        })
        .unwrap();

    let to_address = astria_address(&[2; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

    state
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();
    state
        .put_bridge_account_withdrawer_address(&bridge_address, bridge_address)
        .unwrap();
    state.put_allowed_fee_asset(&asset).unwrap();
    // Put plenty of balance
    state
        .put_account_balance(&bridge_address, &asset, 3 * transfer_amount)
        .unwrap();

    let bridge_unlock_first = BridgeUnlock {
        to: to_address,
        amount: transfer_amount,
        fee_asset: asset.clone(),
        memo: String::new(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    };
    let bridge_unlock_second = BridgeUnlock {
        rollup_block_number: 10,
        ..bridge_unlock_first.clone()
    };

    // first should succeed, next should fail due to duplicate event.
    bridge_unlock_first
        .check_and_execute(&mut state)
        .await
        .unwrap();
    assert_eyre_error(
        &bridge_unlock_second
            .check_and_execute(&mut state)
            .await
            .unwrap_err(),
        "withdrawal event already processed",
    );
}
