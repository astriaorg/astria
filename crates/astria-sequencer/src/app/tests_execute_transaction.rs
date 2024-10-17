use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::{
        fees::v1::{
            InitBridgeAccountFeeComponents,
            RollupDataSubmissionFeeComponents,
        },
        genesis::v1::GenesisAppState,
        transaction::v1::{
            action::{
                BridgeLock,
                BridgeUnlock,
                IbcRelayerChange,
                IbcSudoChange,
                RollupDataSubmission,
                SudoAddressChange,
                Transfer,
                ValidatorUpdate,
            },
            Action,
            TransactionBody,
        },
    },
    sequencerblock::v1::block::Deposit,
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
        benchmark_and_test_utils::{
            BOB_ADDRESS,
            CAROL_ADDRESS,
        },
        test_utils::{
            get_bridge_signing_key,
            initialize_app,
        },
        ActionHandler as _,
    },
    authority::StateReadExt as _,
    benchmark_and_test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
        verification_key,
        ASTRIA_PREFIX,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::StateReadExt as _,
    test_utils::calculate_rollup_data_submission_fee_from_state,
    transaction::{
        InvalidChainId,
        InvalidNonce,
    },
    utils::create_deposit_event,
};

fn proto_genesis_state() -> astria_core::generated::protocol::genesis::v1::GenesisAppState {
    astria_core::generated::protocol::genesis::v1::GenesisAppState {
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
        ..crate::app::benchmark_and_test_utils::proto_genesis_state()
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
    let tx = TransactionBody::builder()
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

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    assert_eq!(
        app.state_delta
            .get_account_balance(&bob_address, &nria())
            .await
            .unwrap(),
        value + 10u128.pow(19)
    );
    let transfer_base = app
        .state_delta
        .get_transfer_fees()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base;
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
            .await
            .unwrap(),
        10u128.pow(19) - (value + transfer_base),
    );
    assert_eq!(
        app.state_delta
            .get_account_nonce(&bob_address)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn app_execute_transaction_transfer_not_native_token() {
    use crate::accounts::StateWriteExt as _;

    let mut app = initialize_app(None, vec![]).await;

    // create some asset to be transferred and update Alice's balance of it
    let value = 333_333;
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    delta_delta
        .put_account_balance(&alice_address, &test_asset(), value)
        .unwrap();
    app.apply(delta_delta);

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let tx = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: bob_address,
                amount: value,
                asset: test_asset(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    assert_eq!(
        app.state_delta
            .get_account_balance(&bob_address, &nria())
            .await
            .unwrap(),
        10u128.pow(19), // genesis balance
    );
    assert_eq!(
        app.state_delta
            .get_account_balance(&bob_address, &test_asset())
            .await
            .unwrap(),
        value, // transferred amount
    );

    let transfer_base = app
        .state_delta
        .get_transfer_fees()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base;
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
            .await
            .unwrap(),
        10u128.pow(19) - transfer_base, // genesis balance - fee
    );
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &test_asset())
            .await
            .unwrap(),
        0, // 0 since all funds of `asset` were transferred
    );

    assert_eq!(
        app.state_delta
            .get_account_nonce(&bob_address)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn app_execute_transaction_transfer_balance_too_low_for_fee() {
    use rand::rngs::OsRng;

    let mut app = initialize_app(None, vec![]).await;

    // create a new key; will have 0 balance
    let keypair = SigningKey::new(OsRng);
    let bob = astria_address_from_hex_string(BOB_ADDRESS);

    // 0-value transfer; only fee is deducted from sender
    let tx = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: bob,
                amount: 0,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&keypair));
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
    let mut app = initialize_app(None, vec![]).await;
    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    delta_delta
        .put_rollup_data_submission_fees(RollupDataSubmissionFeeComponents {
            base: 0,
            multiplier: 1,
        })
        .unwrap();
    app.apply(delta_delta);

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());
    let data = Bytes::from_static(b"hello world");
    let fee = calculate_rollup_data_submission_fee_from_state(&data, &app.state_delta).await;

    let tx = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
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

    let tx = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: test_asset(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_validator_update() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let update = ValidatorUpdate {
        power: 100,
        verification_key: verification_key(1),
    };

    let tx = TransactionBody::builder()
        .actions(vec![Action::ValidatorUpdate(update.clone())])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    let validator_updates = app.state_delta.get_validator_updates().await.unwrap();
    assert_eq!(validator_updates.len(), 1);
    assert_eq!(
        validator_updates.get(verification_key(1).address_bytes()),
        Some(&update)
    );
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_addition() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = TransactionBody::builder()
        .actions(vec![Action::IbcRelayerChange(IbcRelayerChange::Addition(
            alice_address,
        ))])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
    assert!(app.state_delta.is_ibc_relayer(alice_address).await.unwrap());
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

    let tx = TransactionBody::builder()
        .actions(vec![IbcRelayerChange::Removal(alice_address).into()])
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
    assert!(!app.state_delta.is_ibc_relayer(alice_address).await.unwrap());
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

    let tx = TransactionBody::builder()
        .actions(vec![IbcRelayerChange::Removal(alice_address).into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_sudo_address_change() {
    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let new_address = astria_address_from_hex_string(BOB_ADDRESS);
    let tx = TransactionBody::builder()
        .actions(vec![Action::SudoAddressChange(SudoAddressChange {
            new_address,
        })])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    let sudo_address = app.state_delta.get_sudo_address().await.unwrap();
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
    let tx = TransactionBody::builder()
        .actions(vec![Action::SudoAddressChange(SudoAddressChange {
            new_address: alice_address,
        })])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
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
    use astria_core::protocol::transaction::v1::action::FeeAssetChange;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = TransactionBody::builder()
        .actions(vec![Action::FeeAssetChange(FeeAssetChange::Addition(
            test_asset(),
        ))])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    assert!(
        app.state_delta
            .is_allowed_fee_asset(&test_asset())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_removal() {
    use astria_core::protocol::transaction::v1::action::FeeAssetChange;

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

    let tx = TransactionBody::builder()
        .actions(vec![Action::FeeAssetChange(FeeAssetChange::Removal(
            test_asset(),
        ))])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    assert!(
        !app.state_delta
            .is_allowed_fee_asset(&test_asset())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_invalid() {
    use astria_core::protocol::transaction::v1::action::FeeAssetChange;

    let alice = get_alice_signing_key();

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let tx = TransactionBody::builder()
        .actions(vec![Action::FeeAssetChange(FeeAssetChange::Removal(
            nria().into(),
        ))])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
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
    use astria_core::protocol::transaction::v1::action::InitBridgeAccount;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(None, vec![]).await;
    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    let fee = 12; // arbitrary
    delta_delta
        .put_init_bridge_account_fees(InitBridgeAccountFeeComponents {
            base: fee,
            multiplier: 0,
        })
        .unwrap();
    app.apply(delta_delta);

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let action = InitBridgeAccount {
        rollup_id,
        asset: nria().into(),
        fee_asset: nria().into(),
        sudo_address: None,
        withdrawer_address: None,
    };

    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));

    let before_balance = app
        .state_delta
        .get_account_balance(&alice_address, &nria())
        .await
        .unwrap();
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        app.state_delta
            .get_bridge_account_rollup_id(&alice_address)
            .await
            .unwrap()
            .unwrap(),
        rollup_id
    );
    assert_eq!(
        app.state_delta
            .get_bridge_account_ibc_asset(&alice_address)
            .await
            .unwrap(),
        nria().to_ibc_prefixed(),
    );
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
            .await
            .unwrap(),
        before_balance - fee,
    );
}

#[tokio::test]
async fn app_execute_transaction_init_bridge_account_account_already_registered() {
    use astria_core::protocol::transaction::v1::action::InitBridgeAccount;

    let alice = get_alice_signing_key();
    let mut app = initialize_app(None, vec![]).await;

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let action = InitBridgeAccount {
        rollup_id,
        asset: nria().into(),
        fee_asset: nria().into(),
        sudo_address: None,
        withdrawer_address: None,
    };
    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let action = InitBridgeAccount {
        rollup_id,
        asset: nria().into(),
        fee_asset: nria().into(),
        sudo_address: None,
        withdrawer_address: None,
    };

    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
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

    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    delta_delta
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    delta_delta
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    app.apply(delta_delta);

    let amount = 100;
    let action = BridgeLock {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));

    let bridge_before_balance = app
        .state_delta
        .get_account_balance(&bridge_address, &nria())
        .await
        .unwrap();

    app.execute_transaction(signed_tx.clone()).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );
    let expected_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: signed_tx.id(),
        source_action_index: starting_index_of_action,
    };

    assert_eq!(
        app.state_delta
            .get_account_balance(&bridge_address, &nria())
            .await
            .unwrap(),
        bridge_before_balance + amount
    );

    let all_deposits = app.state_delta.get_cached_block_deposits();
    let deposits = all_deposits.get(&rollup_id).unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0], expected_deposit);
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_action_invalid_for_eoa() {
    use astria_core::protocol::transaction::v1::action::BridgeLock;

    let alice = get_alice_signing_key();
    let mut app = initialize_app(None, vec![]).await;

    // don't actually register this address as a bridge address
    let bridge_address = astria_address(&[99; 20]);

    let amount = 100;
    let action = BridgeLock {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_invalid_nonce() {
    let mut app = initialize_app(None, vec![]).await;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    // create tx with invalid nonce 1
    let data = Bytes::from_static(b"hello world");

    let tx = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .nonce(1)
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
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
    let tx = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("wrong-chain")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        app.state_delta
            .get_account_balance(&alice_address, &nria())
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
    let keypair_address = astria_address(keypair.verification_key().address_bytes());

    // figure out needed fee for a single transfer
    let data = Bytes::from_static(b"hello world");
    let fee =
        calculate_rollup_data_submission_fee_from_state(&data, &app.state_delta.clone()).await;

    // transfer just enough to cover single sequence fee with data
    let signed_tx = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: keypair_address,
                amount: fee,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);
    app.execute_transaction(Arc::new(signed_tx)).await.unwrap();

    // build double transfer exceeding balance
    let signed_tx_fail = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset: nria().into(),
            }
            .into(),
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&keypair);
    // try double, see fails stateful check
    let res = signed_tx_fail
        .check_and_execute(Arc::get_mut(&mut app.state_delta).unwrap())
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("insufficient funds for asset"));

    // build single transfer to see passes
    let signed_tx_pass = TransactionBody::builder()
        .actions(vec![
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&keypair);

    signed_tx_pass
        .check_and_execute(Arc::get_mut(&mut app.state_delta).unwrap())
        .await
        .expect("stateful check should pass since we transferred enough to cover fee");
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_unlock_action_ok() {
    use crate::accounts::StateWriteExt as _;

    let alice = get_alice_signing_key();
    let alice_address = astria_address(&alice.address_bytes());

    let mut app = initialize_app(None, vec![]).await;
    let mut delta_delta = StateDelta::new(app.state_delta.clone());

    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());
    let rollup_id: RollupId = RollupId::from_unhashed_bytes(b"testchainid");

    // give bridge eoa funds so it can pay for the
    // unlock transfer action
    let transfer_base = app
        .state_delta
        .get_transfer_fees()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base;
    delta_delta
        .put_account_balance(&bridge_address, &nria(), transfer_base)
        .unwrap();

    // create bridge account
    delta_delta
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    delta_delta
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    delta_delta
        .put_bridge_account_withdrawer_address(&bridge_address, bridge_address)
        .unwrap();
    app.apply(delta_delta);

    let amount = 100;
    let action = BridgeLock {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));

    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    // see can unlock through bridge unlock
    let action = BridgeUnlock {
        to: alice_address,
        amount,
        fee_asset: nria().into(),
        memo: "{ \"msg\": \"lilywashere\" }".into(),
        bridge_address,
        rollup_block_number: 1,
        rollup_withdrawal_event_id: "id-from-rollup".to_string(),
    };

    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&bridge));
    app.execute_transaction(signed_tx)
        .await
        .expect("executing bridge unlock action should succeed");
    assert_eq!(
        app.state_delta
            .get_account_balance(&bridge_address, &nria())
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

    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    delta_delta
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    delta_delta
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    app.apply(delta_delta);

    let amount = 100;
    let action = BridgeLock {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };

    let tx = TransactionBody::builder()
        .actions(vec![action.clone().into(), action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx.clone()).await.unwrap();
    assert_eq!(
        app.state_delta
            .get_account_nonce(&alice_address)
            .await
            .unwrap(),
        1
    );

    let all_deposits = app.state_delta.get_cached_block_deposits();
    let deposits = all_deposits.get(&rollup_id).unwrap();
    assert_eq!(deposits.len(), 2);
    assert_eq!(deposits[0].source_action_index, starting_index_of_action);
    assert_eq!(
        deposits[1].source_action_index,
        starting_index_of_action + 1
    );
}

#[tokio::test]
async fn transaction_execution_records_deposit_event() {
    let mut app = initialize_app(None, vec![]).await;
    let mut delta_delta = app
        .state_delta
        .try_begin_transaction()
        .expect("state Arc should be present and unique");

    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    delta_delta
        .put_bridge_account_rollup_id(&bob_address, [0; 32].into())
        .unwrap();
    delta_delta.put_allowed_fee_asset(&nria()).unwrap();
    delta_delta
        .put_bridge_account_ibc_asset(&bob_address, nria())
        .unwrap();

    let action = BridgeLock {
        to: bob_address,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "test_chain_address".to_string(),
    };
    let tx = TransactionBody::builder()
        .actions(vec![action.into()])
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));

    let expected_deposit = Deposit {
        bridge_address: bob_address,
        rollup_id: [0; 32].into(),
        amount: 1,
        asset: nria().into(),
        destination_chain_address: "test_chain_address".to_string(),
        source_transaction_id: signed_tx.id(),
        source_action_index: 0,
    };
    let expected_deposit_event = create_deposit_event(&expected_deposit);

    signed_tx.check_and_execute(&mut delta_delta).await.unwrap();
    let events = &delta_delta.apply().1;
    let event = events
        .iter()
        .find(|event| event.kind == "tx.deposit")
        .expect("should have deposit event");
    assert_eq!(*event, expected_deposit_event);
}

#[tokio::test]
async fn app_execute_transaction_ibc_sudo_change() {
    let alice = get_alice_signing_key();

    let mut app = initialize_app(Some(genesis_state()), vec![]).await;

    let new_address = astria_address_from_hex_string(BOB_ADDRESS);

    let tx = TransactionBody::builder()
        .actions(vec![Action::IbcSudoChange(IbcSudoChange {
            new_address,
        })])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    app.execute_transaction(signed_tx).await.unwrap();

    let ibc_sudo_address = app.state_delta.get_ibc_sudo_address().await.unwrap();
    assert_eq!(ibc_sudo_address, new_address.bytes());
}

#[tokio::test]
async fn app_execute_transaction_ibc_sudo_change_error() {
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

    let tx = TransactionBody::builder()
        .actions(vec![Action::IbcSudoChange(IbcSudoChange {
            new_address: alice_address,
        })])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx.sign(&alice));
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("signer is not the sudo key"));
}

#[tokio::test]
async fn transaction_execution_records_fee_event() {
    let mut app = initialize_app(None, vec![]).await;

    // transfer funds from Alice to Bob
    let alice = get_alice_signing_key();
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let value = 333_333;
    let tx = TransactionBody::builder()
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
    let signed_tx = Arc::new(tx.sign(&alice));
    let events = app.execute_transaction(signed_tx).await.unwrap();

    let event = events.first().unwrap();
    assert_eq!(event.kind, "tx.fees");
    assert_eq!(event.attributes[0].key, "actionName");
    assert_eq!(event.attributes[1].key, "asset");
    assert_eq!(event.attributes[2].key, "feeAmount");
    assert_eq!(event.attributes[3].key, "positionInTransaction");
}
