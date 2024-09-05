use astria_core::{
    primitive::v1::{
        asset::{
            Denom,
            TracePrefixed,
        },
        Address,
        RollupId,
    },
    protocol::{
        genesis::v1alpha1::Fees,
        transaction::v1alpha1::{
            action::{
                BridgeLockAction,
                BridgeSudoChangeAction,
                BridgeUnlockAction,
                FeeAssetChangeAction,
                FeeChange,
                FeeChangeAction,
                Ics20Withdrawal,
                InitBridgeAccountAction,
                SequenceAction,
                SudoAddressChangeAction,
                TransferAction,
            },
            TransactionParams,
            UnsignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::{
    Snapshot,
    StateDelta,
    StateWrite,
};
use ibc_types::core::client::Height;

use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt as _,
    },
    app::test_utils::{
        get_alice_signing_key,
        ALICE_ADDRESS,
        BOB_ADDRESS,
        CAROL_ADDRESS,
    },
    assets::StateWriteExt as _,
    authority::StateWriteExt as _,
    bridge::StateWriteExt as _,
    ibc::StateWriteExt as _,
    sequence::{
        action::calculate_fee as sequence_calculate_fee,
        StateWriteExt as _,
    },
    test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
    },
    transaction::{
        fees::{
            construct_tx_fee_event,
            get_and_report_tx_fees,
            pay_fees,
            PaymentMapKey,
        },
        StateWriteExt as _,
    },
};

#[tokio::test]
async fn correct_transfer_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let transfer_action = TransferAction {
        to: bob_address,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            transfer_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::TransferBaseFee,
                new_value: fees.transfer_base_fee + 1,
            }
            .into(),
            transfer_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();
    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.transfer_base_fee * 2 + 1);

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            fees.transfer_base_fee,
            "TransferAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            fees.transfer_base_fee + 1,
            "TransferAction".to_string(),
            2,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.transfer_base_fee * 2) - 1
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        (fees.transfer_base_fee * 2) + 1
    );
}

#[tokio::test]
async fn correct_sequence_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let sequence_action = SequenceAction {
        rollup_id: RollupId::new([0; 32]),
        data: vec![0; 32].into(),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            sequence_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::SequenceBaseFee,
                new_value: fees.sequence_base_fee + 1,
            }
            .into(),
            FeeChangeAction {
                fee_change: FeeChange::SequenceByteCostMultiplier,
                new_value: fees.sequence_byte_cost_multiplier + 1,
            }
            .into(),
            sequence_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();
    let expected_fees_sequence_action_1 = sequence_calculate_fee(
        &[0; 32],
        fees.sequence_byte_cost_multiplier,
        fees.sequence_base_fee,
    )
    .unwrap();
    let expected_fees_sequence_action_2 = sequence_calculate_fee(
        &[0; 32],
        fees.sequence_byte_cost_multiplier + 1,
        fees.sequence_base_fee + 1,
    )
    .unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(
        fee_info.amt,
        expected_fees_sequence_action_1 + expected_fees_sequence_action_2
    );

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            expected_fees_sequence_action_1,
            "SequenceAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            expected_fees_sequence_action_2,
            "SequenceAction".to_string(),
            3,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - expected_fees_sequence_action_1 - expected_fees_sequence_action_2
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        expected_fees_sequence_action_1 + expected_fees_sequence_action_2
    );
}

#[tokio::test]
async fn correct_ics20_withdrawal_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let ics20_withdrawal = Ics20Withdrawal {
        amount: 1,
        denom: nria().into(),
        bridge_address: None,
        destination_chain_address: "test".to_string(),
        return_address: astria_address(&[0; 20]),
        timeout_height: Height::new(1, 1).unwrap(),
        timeout_time: 1,
        source_channel: "channel-0".to_string().parse().unwrap(),
        fee_asset: nria().into(),
        memo: String::new(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            ics20_withdrawal.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::Ics20WithdrawalBaseFee,
                new_value: fees.ics20_withdrawal_base_fee + 1,
            }
            .into(),
            ics20_withdrawal.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.ics20_withdrawal_base_fee * 2 + 1);

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            fees.ics20_withdrawal_base_fee,
            "Ics20WithdrawalAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            fees.ics20_withdrawal_base_fee + 1,
            "Ics20WithdrawalAction".to_string(),
            2,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.ics20_withdrawal_base_fee * 2) - 1
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.ics20_withdrawal_base_fee * 2 + 1
    );
}

#[tokio::test]
async fn correct_init_bridge_account_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let init_bridge_account_action = InitBridgeAccountAction {
        rollup_id: RollupId::new([0; 32]),
        asset: nria().into(),
        fee_asset: nria().into(),
        sudo_address: None,
        withdrawer_address: None,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            init_bridge_account_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::InitBridgeAccountBaseFee,
                new_value: fees.init_bridge_account_base_fee + 1,
            }
            .into(),
            init_bridge_account_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.init_bridge_account_base_fee * 2 + 1);

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            fees.init_bridge_account_base_fee,
            "InitBridgeAccountAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            fees.init_bridge_account_base_fee + 1,
            "InitBridgeAccountAction".to_string(),
            2,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.init_bridge_account_base_fee * 2) - 1
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.init_bridge_account_base_fee * 2 + 1
    );
}

#[tokio::test]
async fn correct_bridge_lock_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let bridge_lock_action = BridgeLockAction {
        to: bob_address,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "test".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            bridge_lock_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::BridgeLockByteCostMultiplier,
                new_value: fees.bridge_lock_byte_cost_multiplier + 1,
            }
            .into(),
            FeeChangeAction {
                fee_change: FeeChange::TransferBaseFee,
                new_value: fees.transfer_base_fee + 1,
            }
            .into(),
            bridge_lock_action.clone().into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let expected_fees_bridge_lock_action_1 = estimate_bridge_lock_fees(
        &bridge_lock_action,
        fees.transfer_base_fee,
        fees.bridge_lock_byte_cost_multiplier,
    );
    let expected_fees_bridge_lock_action_2 = estimate_bridge_lock_fees(
        &bridge_lock_action,
        fees.transfer_base_fee + 1,
        fees.bridge_lock_byte_cost_multiplier + 1,
    );

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(
        fee_info.amt,
        expected_fees_bridge_lock_action_1 + expected_fees_bridge_lock_action_2
    );

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            expected_fees_bridge_lock_action_1,
            "BridgeLockAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            expected_fees_bridge_lock_action_2,
            "BridgeLockAction".to_string(),
            3,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - expected_fees_bridge_lock_action_1 - expected_fees_bridge_lock_action_2
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        expected_fees_bridge_lock_action_1 + expected_fees_bridge_lock_action_2
    );
}

#[tokio::test]
async fn correct_bridge_unlock_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let bridge_unlock_action = BridgeUnlockAction {
        to: bob_address,
        amount: 1,
        fee_asset: nria().into(),
        memo: "{ \"msg\": \"ethanwashere\" }".into(),
        bridge_address: bob_address,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            bridge_unlock_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::TransferBaseFee,
                new_value: fees.transfer_base_fee + 1,
            }
            .into(),
            bridge_unlock_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.transfer_base_fee * 2 + 1);

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            fees.transfer_base_fee,
            "BridgeUnlockAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            fees.transfer_base_fee + 1,
            "BridgeUnlockAction".to_string(),
            2,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.transfer_base_fee * 2) - 1
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.transfer_base_fee * 2 + 1
    );
}

#[tokio::test]
async fn correct_bridge_sudo_change_fee_payment_and_event_with_fee_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);

    let bridge_sudo_change_action = BridgeSudoChangeAction {
        bridge_address: bob_address,
        new_sudo_address: Some(bob_address),
        new_withdrawer_address: Some(bob_address),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            bridge_sudo_change_action.clone().into(),
            FeeChangeAction {
                fee_change: FeeChange::BridgeSudoChangeBaseFee,
                new_value: fees.bridge_sudo_change_fee + 1,
            }
            .into(),
            bridge_sudo_change_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let key = fee_payment_map.keys().next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();

    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: bob_address.address_bytes(),
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.bridge_sudo_change_fee * 2 + 1);

    let expected_fee_events = vec![
        construct_tx_fee_event(
            &nria(),
            fees.bridge_sudo_change_fee,
            "BridgeSudoChangeAction".to_string(),
            0,
        ),
        construct_tx_fee_event(
            &nria(),
            fees.bridge_sudo_change_fee + 1,
            "BridgeSudoChangeAction".to_string(),
            2,
        ),
    ];
    assert_eq!(fee_info.events, expected_fee_events);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.bridge_sudo_change_fee * 2) - 1
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.bridge_sudo_change_fee * 2 + 1
    );
}

#[tokio::test]
async fn should_exit_on_invalid_asset_type() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: 1,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);
    state_tx
        .put_sudo_address(bob_address.address_bytes())
        .unwrap();
    put_all_base_fees(&mut state_tx);

    let err = get_and_report_tx_fees(&tx, &state_tx, true)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(err.contains("asset type not allowed for fee payment:"));
}

#[tokio::test]
async fn should_exit_on_missing_sudo_address() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);
    put_all_base_fees(&mut state_tx);

    let err = get_and_report_tx_fees(&tx, &state_tx, true)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(err.contains("sudo key not found"));
}

#[tokio::test]
async fn should_exit_on_missing_source() {
    let mut state_tx = new_state_tx().await;

    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![],
    };

    state_tx
        .put_sudo_address(bob_address.address_bytes())
        .unwrap();
    state_tx.put_allowed_fee_asset(&nria());
    put_all_base_fees(&mut state_tx);

    let err = get_and_report_tx_fees(&tx, &state_tx, true)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(err.contains("failed to get payer address"));
}

#[tokio::test]
async fn should_exit_on_missing_fees() {
    let state_tx = new_state_tx().await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![],
    };

    let err = get_and_report_tx_fees(&tx, &state_tx, true)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(err.contains("transfer base fee not set"));
}

#[tokio::test]
async fn handles_mid_tx_sudo_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let carol_address = astria_address_from_hex_string(CAROL_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);
    state_tx
        .put_account_balance(carol_address, nria(), 0)
        .unwrap();

    let transfer_action = TransferAction {
        to: bob_address,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            transfer_action.clone().into(),
            SudoAddressChangeAction {
                new_address: carol_address,
            }
            .into(),
            transfer_action.into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();
    let expected_fee_event_bob = construct_tx_fee_event(
        &nria(),
        fees.transfer_base_fee,
        "TransferAction".to_string(),
        0,
    );
    let expected_fee_event_carol = construct_tx_fee_event(
        &nria(),
        fees.transfer_base_fee,
        "TransferAction".to_string(),
        2,
    );
    let mut keys = fee_payment_map.keys();

    // Test first fee payment
    let key = keys.next().unwrap();
    let cur_to_address = key.to;
    let fee_info = fee_payment_map.get(key).unwrap();

    // Sometimes, the order of the keys is different. This doesn't impact calculating fees, but we
    // need to check both cases here.
    let (next_address, cur_expected_fee_event, next_expected_fee_event) =
        match cur_to_address.as_slice() {
            addr if addr == bob_address.address_bytes() => (
                carol_address.address_bytes(),
                &expected_fee_event_bob,
                &expected_fee_event_carol,
            ),
            addr if addr == carol_address.address_bytes() => (
                bob_address.address_bytes(),
                &expected_fee_event_carol,
                &expected_fee_event_bob,
            ),
            _ => panic!("unexpected `to` address"),
        };
    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: cur_to_address,
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.transfer_base_fee);
    assert_eq!(fee_info.events[0], *cur_expected_fee_event);

    let key = keys.next().unwrap();
    let fee_info = fee_payment_map.get(key).unwrap();
    assert_eq!(
        *key,
        PaymentMapKey {
            from: alice.address_bytes(),
            to: next_address,
            asset: Denom::from(nria())
        }
    );
    assert_eq!(fee_info.amt, fees.transfer_base_fee);
    assert_eq!(fee_info.events[0], *next_expected_fee_event);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - (fees.transfer_base_fee * 2)
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.transfer_base_fee
    );
    assert_eq!(
        state_tx
            .get_account_balance(carol_address, nria())
            .await
            .unwrap(),
        fees.transfer_base_fee
    );
}

#[tokio::test]
async fn handles_mid_tx_fee_asset_change() {
    let mut state_tx = new_state_tx().await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address_from_hex_string(ALICE_ADDRESS);
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);

    let fees = put_all_base_fees(&mut state_tx);
    initialize_default_state(&mut state_tx, alice_address, bob_address);
    let mock_asset: TracePrefixed = "mock_asset".parse().unwrap();
    state_tx
        .put_account_balance(alice_address, mock_asset.clone(), 10000)
        .unwrap();

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: 1,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
            FeeAssetChangeAction::Addition(mock_asset.clone().into()).into(),
            TransferAction {
                to: bob_address,
                amount: 1,
                asset: nria().into(),
                fee_asset: mock_asset.clone().into(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.clone().into_signed(&alice);
    state_tx.put_current_source(&signed_tx);

    let (_, fee_payment_map) = get_and_report_tx_fees(&tx, &state_tx, true).await.unwrap();
    let fee_payment_map = fee_payment_map.unwrap();

    // Check nria fee payment
    let key = PaymentMapKey {
        from: alice.address_bytes(),
        to: bob_address.address_bytes(),
        asset: Denom::from(nria()),
    };
    let fee_info = fee_payment_map.get(&key).unwrap();
    assert_eq!(fee_info.amt, fees.transfer_base_fee);
    let expected_fee_event_nria = construct_tx_fee_event(
        &nria(),
        fees.transfer_base_fee,
        "TransferAction".to_string(),
        0,
    );
    assert_eq!(fee_info.events[0], expected_fee_event_nria);

    // Check mock_asset fee payment
    // Check nria fee payment
    let key = PaymentMapKey {
        from: alice.address_bytes(),
        to: bob_address.address_bytes(),
        asset: Denom::from(mock_asset.clone()),
    };
    let fee_info = fee_payment_map.get(&key).unwrap();
    assert_eq!(fee_info.amt, fees.transfer_base_fee);
    let expected_fee_event_mock_asset = construct_tx_fee_event(
        &mock_asset,
        fees.transfer_base_fee,
        "TransferAction".to_string(),
        2,
    );
    assert_eq!(fee_info.events[0], expected_fee_event_mock_asset);

    pay_fees(&mut state_tx, fee_payment_map).await.unwrap();

    assert_eq!(
        state_tx
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10000 - fees.transfer_base_fee
    );
    assert_eq!(
        state_tx
            .get_account_balance(alice_address, mock_asset.clone())
            .await
            .unwrap(),
        10000 - fees.transfer_base_fee
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        fees.transfer_base_fee
    );
    assert_eq!(
        state_tx
            .get_account_balance(bob_address, mock_asset)
            .await
            .unwrap(),
        fees.transfer_base_fee
    );
}

fn put_all_base_fees<S: StateWrite>(state: &mut S) -> Fees {
    state.put_transfer_base_fee(1).unwrap();
    state.put_sequence_action_base_fee(2);
    state.put_sequence_action_byte_cost_multiplier(3);
    state.put_init_bridge_account_base_fee(4);
    state.put_bridge_lock_byte_cost_multiplier(5);
    state.put_bridge_sudo_change_base_fee(6);
    state.put_ics20_withdrawal_base_fee(7).unwrap();

    Fees {
        transfer_base_fee: 1,
        sequence_base_fee: 2,
        sequence_byte_cost_multiplier: 3,
        init_bridge_account_base_fee: 4,
        bridge_lock_byte_cost_multiplier: 5,
        bridge_sudo_change_fee: 6,
        ics20_withdrawal_base_fee: 7,
    }
}

async fn new_state_tx() -> StateDelta<Snapshot> {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    StateDelta::new(snapshot)
}

fn initialize_default_state(
    state_tx: &mut StateDelta<Snapshot>,
    alice_address: Address,
    bob_address: Address,
) {
    state_tx
        .put_sudo_address(bob_address.address_bytes())
        .unwrap();
    state_tx.put_allowed_fee_asset(&nria());
    state_tx
        .put_account_balance(alice_address, nria(), 10000)
        .unwrap();
    state_tx
        .put_account_balance(bob_address, nria(), 0)
        .unwrap();
}

fn estimate_bridge_lock_fees(
    act: &BridgeLockAction,
    transfer_base_fee: u128,
    bridge_lock_byte_cost_multiplier: u128,
) -> u128 {
    transfer_base_fee.saturating_add(
        crate::bridge::get_deposit_byte_len(&Deposit::new(
            act.to,
            RollupId::from_unhashed_bytes([0; 32]),
            act.amount,
            act.asset.clone(),
            act.destination_chain_address.clone(),
        ))
        .saturating_mul(bridge_lock_byte_cost_multiplier),
    )
}
