use std::sync::Arc;

use astria_core::{
    primitive::v1::RollupId,
    protocol::{
        fees::v1alpha1::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            InitBridgeAccountFeeComponents,
            SequenceFeeComponents,
            TransferFeeComponents,
        },
        transaction::v1alpha1::{
            action::{
                BridgeLock,
                BridgeSudoChange,
                InitBridgeAccount,
                Sequence,
                Transfer,
            },
            UnsignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::StateDelta;

use super::base_deposit_fee;
use crate::{
    accounts::StateWriteExt as _,
    app::{
        test_utils::{
            get_alice_signing_key,
            get_bridge_signing_key,
            initialize_app_with_storage,
            BOB_ADDRESS,
        },
        ActionHandler,
    },
    bridge::StateWriteExt as _,
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    test_utils::{
        astria_address,
        astria_address_from_hex_string,
        calculate_sequence_action_fee_from_state,
        nria,
    },
};

#[tokio::test]
async fn ensure_correct_block_fees_transfer() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);
    let transfer_base = 1;
    state
        .put_transfer_fees(TransferFeeComponents {
            base: transfer_base,
            multiplier: 0,
        })
        .unwrap();

    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let actions = vec![
        Transfer {
            to: bob_address,
            amount: 1000,
            asset: nria().into(),
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, transfer_base);
}

#[tokio::test]
async fn ensure_correct_block_fees_sequence() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);
    state
        .put_sequence_fees(SequenceFeeComponents {
            base: 1,
            multiplier: 1,
        })
        .unwrap();

    let alice = get_alice_signing_key();
    let data = b"hello world".to_vec();

    let actions = vec![
        Sequence {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: data.clone().into(),
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();
    let total_block_fees: u128 = state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    let expected_fees = calculate_sequence_action_fee_from_state(&data, &state).await;
    assert_eq!(total_block_fees, expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_init_bridge_acct() {
    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);
    let init_bridge_account_base = 1;
    state
        .put_init_bridge_account_fees(InitBridgeAccountFeeComponents {
            base: init_bridge_account_base,
            multiplier: 0,
        })
        .unwrap();

    let alice = get_alice_signing_key();

    let actions = vec![
        InitBridgeAccount {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        }
        .into(),
    ];

    let tx = UnsignedTransaction::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, init_bridge_account_base);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_lock() {
    let alice = get_alice_signing_key();
    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    let transfer_base = 1;
    let bridge_lock_byte_cost_multiplier = 1;

    state
        .put_transfer_fees(TransferFeeComponents {
            base: transfer_base,
            multiplier: 0,
        })
        .unwrap();
    state
        .put_bridge_lock_fees(BridgeLockFeeComponents {
            base: transfer_base,
            multiplier: bridge_lock_byte_cost_multiplier,
        })
        .unwrap();
    state
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();

    let actions = vec![
        BridgeLock {
            to: bridge_address,
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: rollup_id.to_string(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let test_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount: 1,
        asset: nria().into(),
        destination_chain_address: rollup_id.to_string(),
        source_transaction_id: signed_tx.id(),
        source_action_index: starting_index_of_action,
    };

    let total_block_fees: u128 = state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    let expected_fees = transfer_base
        + (base_deposit_fee(&test_deposit.asset, &test_deposit.destination_chain_address)
            * bridge_lock_byte_cost_multiplier);
    assert_eq!(total_block_fees, expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_sudo_change() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());

    let (_, storage) = initialize_app_with_storage(None, vec![]).await;
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    let sudo_change_base = 1;
    state
        .put_bridge_sudo_change_fees(BridgeSudoChangeFeeComponents {
            base: sudo_change_base,
            multiplier: 0,
        })
        .unwrap();
    state
        .put_bridge_account_sudo_address(&bridge_address, alice_address)
        .unwrap();
    state
        .increase_balance(&bridge_address, &nria(), 1)
        .await
        .unwrap();

    let actions = vec![
        BridgeSudoChange {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, sudo_change_base);
}

// TODO(https://github.com/astriaorg/astria/issues/1382): Add test to ensure correct block fees for ICS20 withdrawal
