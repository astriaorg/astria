use astria_core::{
    primitive::v1::asset,
    protocol::{
        fees::v1::TransferFeeComponents,
        transaction::v1::action::{
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            SudoAddressChange,
            ValidatorUpdate,
        },
    },
};
use cnidarium::StateDelta;

use crate::{
    address::StateWriteExt as _,
    app::{
        benchmark_and_test_utils::{
            initialize_app_with_storage,
            ALICE_ADDRESS,
            BOB_ADDRESS,
        },
        test_utils::get_alice_signing_key,
    },
    authority::StateWriteExt as _,
    authorization::AuthorizationHandler,
    benchmark_and_test_utils::{
        assert_eyre_error,
        astria_address,
        astria_address_from_hex_string,
        ASTRIA_PREFIX,
    },
    bridge::StateWriteExt as _,
    ibc::StateWriteExt as _,
};

fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}

#[tokio::test]
async fn ensure_sudo_change_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write sudo address");

    let sudo_change = SudoAddressChange {
        new_address: astria_address_from_hex_string(BOB_ADDRESS),
    };

    assert_eyre_error(
        &sudo_change
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "signer is not the sudo key",
    );
    assert!(
        sudo_change
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn ensure_ibc_sudo_change_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write sudo address");

    let ibc_sudo_change = IbcSudoChange {
        new_address: astria_address_from_hex_string(BOB_ADDRESS),
    };

    assert_eyre_error(
        &ibc_sudo_change
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "signer is not the sudo key",
    );
    assert!(
        ibc_sudo_change
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn ensure_validator_update_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write sudo address");

    let validator_update = ValidatorUpdate {
        verification_key: get_alice_signing_key().verification_key(),
        power: 1,
    };

    assert_eyre_error(
        &validator_update
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "signer is not the sudo key",
    );
    assert!(
        validator_update
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn ensure_fee_asset_change_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write sudo address");

    let fee_asset_change = FeeAssetChange::Addition(test_asset());

    assert_eyre_error(
        &fee_asset_change
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "signer is not the sudo key",
    );
    assert!(
        fee_asset_change
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn ensure_fee_change_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write sudo address");

    let fee_change = FeeChange::Transfer(TransferFeeComponents {
        base: 10,
        multiplier: 0,
    });

    assert_eyre_error(
        &fee_change
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "signer is not the sudo key",
    );
    assert!(
        fee_change
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn ensure_ibc_relayer_change_action_is_authorized() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state
        .put_ibc_sudo_address(astria_address_from_hex_string(ALICE_ADDRESS))
        .expect("failed to write IBC relayer address");

    let ibc_relayer_change =
        IbcRelayerChange::Addition(astria_address_from_hex_string(BOB_ADDRESS));

    assert_eyre_error(
        &ibc_relayer_change
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "unauthorized address for IBC relayer change",
    );
    assert!(
        ibc_relayer_change
            .check_authorization(&state, &astria_address_from_hex_string(ALICE_ADDRESS))
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}

#[tokio::test]
async fn bridge_unlock_no_withdrawer_address_fails() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let asset = test_asset();
    let transfer_amount = 100;

    let to_address = astria_address(&[2; 20]);
    let bridge_address = astria_address(&[3; 20]);
    state
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();

    let bridge_unlock: BridgeUnlock = BridgeUnlock {
        to: to_address,
        amount: transfer_amount,
        fee_asset: asset.clone(),
        memo: String::new(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    };

    // missing withdrawer address should cause failure
    assert_eyre_error(
        &bridge_unlock
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "bridge account does not have an associated withdrawer address",
    );
}

#[tokio::test]
async fn ensure_bridge_unlock_withdrawer_address_is_authorized() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let asset = test_asset();
    let transfer_amount = 100;

    let bridge_address = astria_address(&[3; 20]);
    let withdrawer_address = astria_address_from_hex_string(ALICE_ADDRESS);
    state
        .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
        .unwrap();

    let bridge_unlock = BridgeUnlock {
        to: astria_address(&[2; 20]),
        amount: transfer_amount,
        fee_asset: asset,
        memo: String::new(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    };

    // invalid sender, doesn't match action's bridge account's withdrawer, should fail
    assert_eyre_error(
        &bridge_unlock
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "unauthorized to unlock bridge account",
    );

    assert!(
        bridge_unlock
            .check_authorization(&state, &withdrawer_address)
            .await
            .is_ok(),
        "correct withdrawer should be ok"
    );
}

#[tokio::test]
async fn ensure_bridge_account_sudo_change_is_authorized() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    let asset = test_asset();

    let bridge_address = astria_address(&[99; 20]);
    let sudo_address = astria_address_from_hex_string(ALICE_ADDRESS);
    state
        .put_bridge_account_sudo_address(&bridge_address, sudo_address)
        .unwrap();

    let action = BridgeSudoChange {
        bridge_address,
        new_sudo_address: None,
        new_withdrawer_address: None,
        fee_asset: asset.clone(),
    };

    assert_eyre_error(
        &action
            .check_authorization(&state, &astria_address_from_hex_string(BOB_ADDRESS))
            .await
            .unwrap_err(),
        "unauthorized for bridge sudo change action",
    );
    assert!(
        action
            .check_authorization(&state, &sudo_address)
            .await
            .is_ok(),
        "correct signer should be ok"
    );
}
