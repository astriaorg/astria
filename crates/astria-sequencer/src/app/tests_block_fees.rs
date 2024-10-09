use std::sync::Arc;

use astria_core::{
    primitive::v1::RollupId,
    protocol::{
        fees::v1alpha1::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            FeeComponentsInner,
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
use tendermint::abci::EventAttributeIndexExt as _;

use crate::{
    accounts::StateWriteExt as _,
    app::test_utils::{
        get_alice_signing_key,
        get_bridge_signing_key,
        initialize_app,
        BOB_ADDRESS,
    },
    authority::StateReadExt as _,
    bridge::StateWriteExt as _,
    fees::{
        calculate_base_deposit_fee,
        calculate_sequence_action_fee_from_state,
        FeeHandler as _,
        StateReadExt as _,
        StateWriteExt as _,
    },
    test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
    },
};

#[tokio::test]
async fn transaction_execution_records_fee_event() {
    let mut app = initialize_app(None, vec![]).await;

    // transfer funds from Alice to Bob
    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let value = 333_333;
    let tx = UnsignedTransaction::builder()
        .actions(vec![
            Transfer {
                to: bob_address,
                amount: value,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.into_signed(&alice));
    let tx_id = signed_tx.id();
    app.execute_transaction(signed_tx).await.unwrap();

    let sudo_address = app.state.get_sudo_address().await.unwrap();
    let end_block = app.end_block(1, &sudo_address).await.unwrap();

    let events = end_block.events;
    let transfer_base_fee = Transfer::fee_components(&app.state).await.unwrap().base_fee;
    let event = events.first().unwrap();
    assert_eq!(event.kind, "tx.fees");
    assert_eq!(
        event.attributes[0],
        ("asset", nria().to_ibc_prefixed().to_string())
            .index()
            .into()
    );
    assert_eq!(
        event.attributes[1],
        ("feeAmount", transfer_base_fee.to_string()).index().into()
    );
    assert_eq!(
        event.attributes[2],
        ("sourceTransactionId", tx_id.to_string(),).index().into()
    );
    assert_eq!(
        event.attributes[3],
        ("sourceActionIndex", "0",).index().into()
    );
}

#[tokio::test]
async fn ensure_correct_block_fees_transfer() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let transfer_base_fee = 1;
    state_tx
        .put_transfer_fees(TransferFeeComponents(FeeComponentsInner {
            base_fee: transfer_base_fee,
            computed_cost_multiplier: 0,
        }))
        .unwrap();
    app.apply(state_tx);

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
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, transfer_base_fee);
}

#[tokio::test]
async fn ensure_correct_block_fees_sequence() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_sequence_fees(SequenceFeeComponents(FeeComponentsInner {
            base_fee: 1,
            computed_cost_multiplier: 1,
        }))
        .unwrap();
    app.apply(state_tx);

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
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    let expected_fees = calculate_sequence_action_fee_from_state(&data, &app.state)
        .await
        .unwrap();
    assert_eq!(total_block_fees, expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_init_bridge_acct() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let init_bridge_account_base_fee = 1;
    state_tx
        .put_init_bridge_account_fees(InitBridgeAccountFeeComponents(FeeComponentsInner {
            base_fee: init_bridge_account_base_fee,
            computed_cost_multiplier: 0,
        }))
        .unwrap();
    app.apply(state_tx);

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
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, init_bridge_account_base_fee);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_lock() {
    let alice = get_alice_signing_key();
    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());

    let transfer_base_fee = 1;
    let bridge_lock_byte_cost_multiplier = 1;

    state_tx
        .put_transfer_fees(TransferFeeComponents(FeeComponentsInner {
            base_fee: transfer_base_fee,
            computed_cost_multiplier: 0,
        }))
        .unwrap();
    state_tx
        .put_bridge_lock_fees(BridgeLockFeeComponents(FeeComponentsInner {
            base_fee: transfer_base_fee,
            computed_cost_multiplier: bridge_lock_byte_cost_multiplier,
        }))
        .unwrap();
    state_tx
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state_tx
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    app.apply(state_tx);

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
    app.execute_transaction(signed_tx.clone()).await.unwrap();

    let test_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount: 1,
        asset: nria().into(),
        destination_chain_address: rollup_id.to_string(),
        source_transaction_id: signed_tx.id(),
        source_action_index: starting_index_of_action,
    };

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    let expected_fees = transfer_base_fee
        + (calculate_base_deposit_fee(&test_deposit).unwrap() * bridge_lock_byte_cost_multiplier);
    assert_eq!(total_block_fees, expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_sudo_change() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());

    let sudo_change_base_fee = 1;
    state_tx
        .put_bridge_sudo_change_fees(BridgeSudoChangeFeeComponents(FeeComponentsInner {
            base_fee: sudo_change_base_fee,
            computed_cost_multiplier: 0,
        }))
        .unwrap();
    state_tx
        .put_bridge_account_sudo_address(&bridge_address, alice_address)
        .unwrap();
    state_tx
        .increase_balance(&bridge_address, &nria(), 1)
        .await
        .unwrap();
    app.apply(state_tx);

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
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .unwrap()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, sudo_change_base_fee);
}

// TODO(https://github.com/astriaorg/astria/issues/1382): Add test to ensure correct block fees for ICS20 withdrawal
