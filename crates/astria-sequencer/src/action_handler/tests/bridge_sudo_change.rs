use astria_core::{
    primitive::v1::TransactionId,
    protocol::{
        fees::v1::BridgeSudoChangeFeeComponents,
        transaction::v1::action::BridgeSudoChange,
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
        astria_address,
        ASTRIA_PREFIX,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    fees::StateWriteExt as _,
    transaction::{
        StateWriteExt as _,
        TransactionContext,
    },
};

#[tokio::test]
async fn bridge_sudo_change_fails_with_unauthorized_if_signer_is_not_sudo_address() {
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
    state.put_allowed_fee_asset(&asset).unwrap();

    let bridge_address = astria_address(&[99; 20]);
    let sudo_address = astria_address(&[98; 20]);
    state
        .put_bridge_account_sudo_address(&bridge_address, sudo_address)
        .unwrap();

    let action = BridgeSudoChange {
        bridge_address,
        new_sudo_address: None,
        new_withdrawer_address: None,
        fee_asset: asset.clone(),
    };

    assert!(
        action
            .check_and_execute(state)
            .await
            .unwrap_err()
            .to_string()
            .contains("unauthorized for bridge sudo change action")
    );
}

#[tokio::test]
async fn bridge_sudo_change_executes_as_expected() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    let sudo_address = astria_address(&[98; 20]);
    state.put_transaction_context(TransactionContext {
        address_bytes: sudo_address.bytes(),
        transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    });
    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
    state
        .put_bridge_sudo_change_fees(BridgeSudoChangeFeeComponents {
            base: 10,
            multiplier: 0,
        })
        .unwrap();

    let fee_asset = test_asset();
    state.put_allowed_fee_asset(&fee_asset).unwrap();

    let bridge_address = astria_address(&[99; 20]);

    state
        .put_bridge_account_sudo_address(&bridge_address, sudo_address)
        .unwrap();

    let new_sudo_address = astria_address(&[98; 20]);
    let new_withdrawer_address = astria_address(&[97; 20]);
    state
        .put_account_balance(&bridge_address, &fee_asset, 10)
        .unwrap();

    let action = BridgeSudoChange {
        bridge_address,
        new_sudo_address: Some(new_sudo_address),
        new_withdrawer_address: Some(new_withdrawer_address),
        fee_asset,
    };

    action.check_and_execute(&mut state).await.unwrap();

    assert_eq!(
        state
            .get_bridge_account_sudo_address(&bridge_address)
            .await
            .unwrap(),
        Some(new_sudo_address.bytes()),
    );
    assert_eq!(
        state
            .get_bridge_account_withdrawer_address(&bridge_address)
            .await
            .unwrap(),
        Some(new_withdrawer_address.bytes()),
    );
}
