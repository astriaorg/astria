use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::{
        genesis::v1alpha1::GenesisAppState,
        transaction::v1alpha1::{
            action::{
                BridgeLockAction,
                BridgeUnlockAction,
                IbcRelayerChangeAction,
                SequenceAction,
                SudoAddressChangeAction,
                TransferAction,
                ValidatorUpdate,
            },
            Action,
            TransactionParams,
            UnsignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::Deposit,
    Protobuf as _,
};
use bytes::Bytes;
use cnidarium::{
    ArcStateDeltaExt as _,
    StateDelta,
};

use super::test_utils::get_alice_signing_key;
use crate::{
    accounts::StateReadExt as _,
    app::{
        test_utils::{
            get_bridge_signing_key,
            initialize_app,
            BOB_ADDRESS,
            CAROL_ADDRESS,
        },
        ActionHandler as _,
    },
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    authority::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::StateReadExt as _,
    sequence::calculate_fee_from_state,
    test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
        ASTRIA_PREFIX,
    },
    transaction::{
        InvalidChainId,
        InvalidNonce,
    },
    utils::create_deposit_event,
};

fn proto_genesis_state() -> astria_core::generated::protocol::genesis::v1alpha1::GenesisAppState {
    astria_core::generated::protocol::genesis::v1alpha1::GenesisAppState {
        authority_sudo_address: Some(
            get_alice_signing_key()
                .try_address(ASTRIA_PREFIX)
                .unwrap()
                .to_raw(),
        ),
        ibc_sudo_address: Some(
            get_alice_signing_key()
                .try_address(ASTRIA_PREFIX)
                .unwrap()
                .to_raw(),
        ),
        ..crate::app::test_utils::proto_genesis_state()
    }
}

fn genesis_state() -> GenesisAppState {
    GenesisAppState::try_from_raw(proto_genesis_state()).unwrap()
}

fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}

#[tokio::test]
async fn app_execute_transaction_transfer() {
    let mut app = initialize_app(None, vec![]).await;

    // transfer funds from Alice to Bob
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
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
                asset: crate::test_utils::nria().into(),
                fee_asset: crate::test_utils::nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    assert_eq!(
        app.state
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        value + 10u128.pow(19)
    );
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10u128.pow(19) - (value + transfer_fee),
    );
    assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
}

#[tokio::test]
async fn app_execute_transaction_transfer_not_native_token() {
    use crate::accounts::StateWriteExt as _;

    let mut app = initialize_app(None, vec![]).await;

    // create some asset to be transferred and update Alice's balance of it
    let value = 333_333;
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_account_balance(alice_address, &test_asset(), value)
        .unwrap();
    app.apply(state_tx);

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: value,
                asset: test_asset(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    assert_eq!(
        app.state
            .get_account_balance(bob_address, nria())
            .await
            .unwrap(),
        10u128.pow(19), // genesis balance
    );
    assert_eq!(
        app.state
            .get_account_balance(bob_address, test_asset())
            .await
            .unwrap(),
        value, // transferred amount
    );

    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10u128.pow(19) - transfer_fee, // genesis balance - fee
    );
    assert_eq!(
        app.state
            .get_account_balance(alice_address, test_asset())
            .await
            .unwrap(),
        0, // 0 since all funds of `asset` were transferred
    );

    assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
}

#[tokio::test]
async fn app_execute_transaction_transfer_balance_too_low_for_fee() {
    use rand::rngs::OsRng;

    let mut app = initialize_app(None, vec![]).await;

    // create a new key; will have 0 balance
    let keypair = SigningKey::new(OsRng);
    let bob = astria_address_from_hex_string(BOB_ADDRESS);

    // 0-value transfer; only fee is deducted from sender
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob,
                amount: 0,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&keypair));
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("insufficient funds"));
}

#[tokio::test]
async fn app_execute_transaction_sequence() {
    use crate::sequence::StateWriteExt as _;

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_sequence_action_base_fee(0);
    state_tx.put_sequence_action_byte_cost_multiplier(1);
    app.apply(state_tx);

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let data = Bytes::from_static(b"hello world");
    let fee = calculate_fee_from_state(&data, &app.state).await.unwrap();

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10u128.pow(19) - fee,
    );
}

#[tokio::test]
async fn app_execute_transaction_invalid_fee_asset() {
    let mut app = initialize_app(None, vec![]).await;

    let alice = get_alice_signing_key();
    let data = Bytes::from_static(b"hello world");

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: test_asset(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_validator_update() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let update = ValidatorUpdate {
        power: 100,
        verification_key: crate::test_utils::verification_key(1),
    };

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::ValidatorUpdate(update.clone())],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    let validator_updates = app.state.get_validator_updates().await.unwrap();
    assert_eq!(validator_updates.len(), 1);
    assert_eq!(
        validator_updates.get(crate::test_utils::verification_key(1).address_bytes()),
        Some(&update)
    );
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_addition() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![IbcRelayerChangeAction::Addition(alice_address).into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert!(app.state.is_ibc_relayer(alice_address).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_deletion() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let genesis_state = {
        let mut state = proto_genesis_state();
        state.ibc_relayer_addresses.push(alice_address.to_raw());
        state
    }
    .try_into()
    .unwrap();
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert!(!app.state.is_ibc_relayer(alice_address).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_invalid() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let genesis_state = {
        let mut state = proto_genesis_state();
        state
            .ibc_sudo_address
            .replace(astria_address(&[0; 20]).to_raw());
        state.ibc_relayer_addresses.push(alice_address.to_raw());
        state
    }
    .try_into()
    .unwrap();
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_sudo_address_change() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let new_address = astria_address_from_hex_string(BOB_ADDRESS);

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
            new_address,
        })],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    let sudo_address = app.state.get_sudo_address().await.unwrap();
    assert_eq!(sudo_address, new_address.bytes());
}

#[tokio::test]
async fn app_execute_transaction_sudo_address_change_error() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let authority_sudo_address = astria_address_from_hex_string(CAROL_ADDRESS);

    let genesis_state = {
        let mut state = proto_genesis_state();
        state
            .authority_sudo_address
            .replace(authority_sudo_address.to_raw());
        state
            .ibc_sudo_address
            .replace(astria_address(&[0u8; 20]).to_raw());
        state
    }
    .try_into()
    .unwrap();
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: alice_address,
        })],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("signer is not the sudo key"));
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_addition() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Addition(
            test_asset(),
        ))],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert!(app.state.is_allowed_fee_asset(&test_asset()).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_removal() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let genesis_state = {
        let mut state = proto_genesis_state();
        state.allowed_fee_assets.push(test_asset().to_string());
        state
    }
    .try_into()
    .unwrap();
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
            test_asset(),
        ))],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert!(!app.state.is_allowed_fee_asset(test_asset()).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_invalid() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

    let alice = get_alice_signing_key();

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
            nria().into(),
        ))],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("cannot remove last allowed fee asset"));
}

#[tokio::test]
async fn app_execute_transaction_init_bridge_account_ok() {
    use astria_core::protocol::transaction::v1alpha1::action::InitBridgeAccountAction;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let fee = 12; // arbitrary
    state_tx.put_init_bridge_account_base_fee(fee);
    app.apply(state_tx);

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let action = InitBridgeAccountAction {
        rollup_id,
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
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));

    let before_balance = app
        .state
        .get_account_balance(alice_address, nria())
        .await
        .unwrap();
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert_eq!(
        app.state
            .get_bridge_account_rollup_id(alice_address)
            .await
            .unwrap()
            .unwrap(),
        rollup_id
    );
    assert_eq!(
        app.state
            .get_bridge_account_ibc_asset(alice_address)
            .await
            .unwrap(),
        nria().to_ibc_prefixed(),
    );
    assert_eq!(
        app.state
            .get_account_balance(alice_address, &nria())
            .await
            .unwrap(),
        before_balance - fee,
    );
}

#[tokio::test]
async fn app_execute_transaction_init_bridge_account_account_already_registered() {
    use astria_core::protocol::transaction::v1alpha1::action::InitBridgeAccountAction;

    let alice = get_alice_signing_key();
    let mut app = initialize_app(None, vec![]).await;

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let action = InitBridgeAccountAction {
        rollup_id,
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

        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let action = InitBridgeAccountAction {
        rollup_id,
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
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_action_ok() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let mut app = initialize_app(None, vec![]).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, nria())
        .unwrap();
    app.apply(state_tx);

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));

    let alice_before_balance = app
        .state
        .get_account_balance(alice_address, nria())
        .await
        .unwrap();
    let bridge_before_balance = app
        .state
        .get_account_balance(bridge_address, nria())
        .await
        .unwrap();

    app.execute_transaction(signed_tx.clone()).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    let expected_deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        nria().into(),
        "nootwashere".to_string(),
        signed_tx.id(),
        starting_index_of_action,
    );

    let fee = transfer_fee
        + app
            .state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .unwrap()
            * crate::bridge::get_deposit_byte_len(&expected_deposit);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        alice_before_balance - (amount + fee)
    );
    assert_eq!(
        app.state
            .get_account_balance(bridge_address, nria())
            .await
            .unwrap(),
        bridge_before_balance + amount
    );

    let deposits = app.state.get_deposit_events(&rollup_id).await.unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0], expected_deposit);
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_action_invalid_for_eoa() {
    use astria_core::protocol::transaction::v1alpha1::action::BridgeLockAction;

    let alice = get_alice_signing_key();
    let mut app = initialize_app(None, vec![]).await;

    // don't actually register this address as a bridge address
    let bridge_address = astria_address(&[99; 20]);

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_invalid_nonce() {
    let mut app = initialize_app(None, vec![]).await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    // create tx with invalid nonce 1
    let data = Bytes::from_static(b"hello world");
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(1)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10u128.pow(19),
    );

    assert_eq!(
        response
            .unwrap_err()
            .downcast_ref::<InvalidNonce>()
            .map(|nonce_err| nonce_err.0)
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn app_execute_transaction_invalid_chain_id() {
    let mut app = initialize_app(None, vec![]).await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    // create tx with invalid nonce 1
    let data = Bytes::from_static(b"hello world");
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("wrong-chain")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, nria())
            .await
            .unwrap(),
        10u128.pow(19),
    );

    assert_eq!(
        response
            .unwrap_err()
            .downcast_ref::<InvalidChainId>()
            .map(|chain_id_err| &chain_id_err.0)
            .unwrap(),
        "wrong-chain"
    );
}

#[tokio::test]
async fn app_stateful_check_fails_insufficient_total_balance() {
    use rand::rngs::OsRng;

    let mut app = initialize_app(None, vec![]).await;

    let alice = get_alice_signing_key();

    // create a new key; will have 0 balance
    let keypair = SigningKey::new(OsRng);
    let keypair_address = astria_address(&keypair.verification_key().address_bytes());

    // figure out needed fee for a single transfer
    let data = Bytes::from_static(b"hello world");
    let fee = calculate_fee_from_state(&data, &app.state.clone())
        .await
        .unwrap();

    // transfer just enough to cover single sequence fee with data
    let signed_tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: keypair_address,
                amount: fee,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&alice);

    // make transfer
    app.execute_transaction(Arc::new(signed_tx)).await.unwrap();

    // build double transfer exceeding balance
    let signed_tx_fail = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset: nria().into(),
            }
            .into(),
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&keypair);

    // try double, see fails stateful check
    let res = signed_tx_fail
        .check_and_execute(Arc::get_mut(&mut app.state).unwrap())
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("insufficient funds for asset"));

    // build single transfer to see passes
    let signed_tx_pass = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&keypair);

    signed_tx_pass
        .check_and_execute(Arc::get_mut(&mut app.state).unwrap())
        .await
        .expect("stateful check should pass since we transferred enough to cover fee");
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_unlock_action_ok() {
    use crate::accounts::StateWriteExt as _;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());

    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());
    let rollup_id: RollupId = RollupId::from_unhashed_bytes(b"testchainid");

    // give bridge eoa funds so it can pay for the
    // unlock transfer action
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    state_tx
        .put_account_balance(bridge_address, nria(), transfer_fee)
        .unwrap();

    // create bridge account
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, nria())
        .unwrap();
    state_tx.put_bridge_account_withdrawer_address(bridge_address, bridge_address);
    app.apply(state_tx);

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));

    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    // see can unlock through bridge unlock
    let action = BridgeUnlockAction {
        to: alice_address,
        amount,
        fee_asset: nria().into(),
        memo: "{ \"msg\": \"lilywashere\" }".into(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "id-from-rollup".to_string(),
    };

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&bridge));
    app.execute_transaction(signed_tx)
        .await
        .expect("executing bridge unlock action should succeed");
    assert_eq!(
        app.state
            .get_account_balance(bridge_address, nria())
            .await
            .expect("executing bridge unlock action should succeed"),
        0,
        "bridge should've transferred out whole balance"
    );
}

#[tokio::test]
async fn app_execute_transaction_action_index_correctly_increments() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let mut app = initialize_app(None, vec![]).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, nria())
        .unwrap();
    app.apply(state_tx);

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![action.clone().into(), action.into()],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));
    app.execute_transaction(signed_tx.clone()).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    let deposits = app.state.get_deposit_events(&rollup_id).await.unwrap();
    assert_eq!(deposits.len(), 2);
    assert_eq!(
        deposits[0].position_in_source_transaction(),
        starting_index_of_action
    );
    assert_eq!(
        deposits[1].position_in_source_transaction(),
        starting_index_of_action + 1
    );
}

#[tokio::test]
async fn transaction_execution_records_deposit_event() {
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = app
        .state
        .try_begin_transaction()
        .expect("state Arc should be present and unique");

    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    state_tx.put_bridge_account_rollup_id(bob_address, &[0; 32].into());
    state_tx.put_allowed_fee_asset(nria());
    state_tx
        .put_bridge_account_ibc_asset(bob_address, nria())
        .unwrap();
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            BridgeLockAction {
                to: bob_address,
                amount: 1,
                asset: nria().into(),
                fee_asset: nria().into(),
                destination_chain_address: "test_chain_address".to_string(),
            }
            .into(),
        ],
    };

    let signed_tx = Arc::new(tx.into_signed(&alice));

    let expected_deposit = Deposit::new(
        bob_address,
        [0; 32].into(),
        1,
        nria().into(),
        "test_chain_address".to_string(),
        signed_tx.id(),
        0,
    );
    let expected_deposit_event = create_deposit_event(&expected_deposit);

    signed_tx.check_and_execute(&mut state_tx).await.unwrap();
    let events = &state_tx.apply().1;
    for event in events {
        if event.kind == "tx.deposit" {
            assert_eq!(*event, expected_deposit_event);
            return;
        }
    }
    panic!("no deposit event found in events");
}
