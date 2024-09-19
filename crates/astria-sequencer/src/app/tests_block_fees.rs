use std::sync::Arc;

use astria_core::{
    primitive::v1::RollupId,
    protocol::transaction::v1alpha1::{
        action::{
            BridgeLockAction,
            BridgeSudoChangeAction,
            InitBridgeAccountAction,
            SequenceAction,
            TransferAction,
        },
        TransactionParams,
        UnsignedTransaction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::StateDelta;
use tendermint::abci::EventAttributeIndexExt as _;

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    app::test_utils::{
        get_alice_signing_key,
        get_bridge_signing_key,
        initialize_app,
        BOB_ADDRESS,
    },
    assets::StateReadExt as _,
    bridge::{
        calculate_base_deposit_fee,
        StateWriteExt as _,
    },
    sequence::{
        calculate_fee_from_state,
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
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: value,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));

    let events = app.execute_transaction(signed_tx).await.unwrap();
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    let event = events.first().unwrap();
    assert_eq!(event.kind, "tx.fees");
    assert_eq!(
        event.attributes[0],
        ("asset", nria().to_string()).index().into()
    );
    assert_eq!(
        event.attributes[1],
        ("feeAmount", transfer_fee.to_string()).index().into()
    );
    assert_eq!(
        event.attributes[2],
        (
            "actionType",
            "astria.protocol.transactions.v1alpha1.TransferAction"
        )
            .index()
            .into()
    );
}

#[tokio::test]
async fn ensure_correct_block_fees_transfer() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let transfer_base_fee = 1;
    state_tx.put_transfer_base_fee(transfer_base_fee).unwrap();
    app.apply(state_tx);

    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let actions = vec![
        TransferAction {
            to: bob_address,
            amount: 1000,
            asset: nria().into(),
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions,
    };
    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, fee)| fee)
        .sum();
    assert_eq!(total_block_fees, transfer_base_fee);
}

#[tokio::test]
async fn ensure_correct_block_fees_sequence() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_sequence_action_base_fee(1);
    state_tx.put_sequence_action_byte_cost_multiplier(1);
    app.apply(state_tx);

    let alice = get_alice_signing_key();
    let data = b"hello world".to_vec();

    let actions = vec![
        SequenceAction {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: data.clone().into(),
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions,
    };
    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, fee)| fee)
        .sum();
    let expected_fees = calculate_fee_from_state(&data, &app.state).await.unwrap();
    assert_eq!(total_block_fees, expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_init_bridge_acct() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let init_bridge_account_base_fee = 1;
    state_tx.put_init_bridge_account_base_fee(init_bridge_account_base_fee);
    app.apply(state_tx);

    let alice = get_alice_signing_key();

    let actions = vec![
        InitBridgeAccountAction {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        }
        .into(),
    ];

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions,
    };
    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, fee)| fee)
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

    state_tx.put_transfer_base_fee(transfer_base_fee).unwrap();
    state_tx.put_bridge_lock_byte_cost_multiplier(bridge_lock_byte_cost_multiplier);
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, nria())
        .unwrap();
    app.apply(state_tx);

    let actions = vec![
        BridgeLockAction {
            to: bridge_address,
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: rollup_id.to_string(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions,
    };
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
        .await
        .unwrap()
        .into_iter()
        .map(|(_, fee)| fee)
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
    state_tx.put_bridge_sudo_change_base_fee(sudo_change_base_fee);
    state_tx.put_bridge_account_sudo_address(bridge_address, alice_address);
    state_tx
        .increase_balance(bridge_address, nria(), 1)
        .await
        .unwrap();
    app.apply(state_tx);

    let actions = vec![
        BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions,
    };
    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let total_block_fees: u128 = app
        .state
        .get_block_fees()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, fee)| fee)
        .sum();
    assert_eq!(total_block_fees, sudo_change_base_fee);
}

// TODO(https://github.com/astriaorg/astria/issues/1382): Add test to ensure correct block fees for ICS20 withdrawal
