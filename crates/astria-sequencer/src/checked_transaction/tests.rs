use astria_core::{
    generated::protocol::transaction::v1::{
        Transaction as RawTransaction,
        TransactionBody as RawTransactionBody,
        TransactionParams as RawTransactionParams,
    },
    primitive::v1::{
        asset::Denom,
        RollupId,
    },
    protocol::transaction::v1::{
        action::{
            BridgeLock,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
        },
        TransactionBodyBuilder,
    },
    sequencerblock::v1::block::Deposit,
};
use prost::Name as _;

use super::*;
use crate::{
    fees::StateReadExt as _,
    test_utils::{
        nria,
        Fixture,
        ALICE,
        ALICE_ADDRESS,
        ALICE_ADDRESS_BYTES,
        BOB_ADDRESS,
        SUDO,
    },
    utils::create_deposit_event,
};

fn test_asset() -> Denom {
    "test".parse().unwrap()
}

#[tokio::test]
async fn should_fail_construction_if_tx_too_large() {
    let fixture = Fixture::default_initialized().await;

    let unchecked_tx = TransactionBodyBuilder::new()
        .chain_id("test".to_string())
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::new([1; 32]),
            data: Bytes::from(vec![1; 256_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .try_build()
        .unwrap()
        .sign(&ALICE);

    let encoded_tx = Bytes::from(unchecked_tx.into_raw().encode_to_vec());
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(error, CheckedTransactionInitialCheckError::TooLarge { .. }),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_construction_if_tx_cannot_be_decoded() {
    let fixture = Fixture::default_initialized().await;

    let encoded_tx = Bytes::from(vec![1, 2, 3]);
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(error, CheckedTransactionInitialCheckError::Decode { .. }),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_construction_if_tx_cannot_be_converted_from_proto() {
    let fixture = Fixture::default_initialized().await;

    let unchecked_tx = TransactionBodyBuilder::new()
        .chain_id("test".to_string())
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::new([1; 32]),
            data: Bytes::from(vec![1, 2, 3]),
            fee_asset: nria().into(),
        }
        .into()])
        .try_build()
        .unwrap()
        .sign(&ALICE);

    let mut raw_tx = unchecked_tx.into_raw();
    raw_tx.public_key = Bytes::from(vec![1, 2, 3]);

    let encoded_tx = Bytes::from(raw_tx.encode_to_vec());
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(error, CheckedTransactionInitialCheckError::Convert { .. }),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_construction_if_no_actions() {
    let fixture = Fixture::default_initialized().await;

    let raw_tx_body = RawTransactionBody {
        params: Some(RawTransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        }),
        actions: vec![],
    };
    let body_bytes = raw_tx_body.encode_to_vec();
    let signature = ALICE.sign(&body_bytes);
    let verification_key = ALICE.verification_key();
    let raw_tx = RawTransaction {
        signature: Bytes::copy_from_slice(&signature.to_bytes()),
        public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
        body: Some(pbjson_types::Any {
            type_url: RawTransactionBody::type_url(),
            value: Bytes::from(body_bytes),
        }),
    };

    let encoded_tx = Bytes::from(raw_tx.encode_to_vec());
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(error, CheckedTransactionInitialCheckError::Convert { .. }),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_construction_if_action_fails_initial_checks() {
    let fixture = Fixture::default_initialized().await;

    // Alice is not sudo address, so this action should fail checks since Alice signs it.
    let unchecked_tx = TransactionBodyBuilder::new()
        .chain_id("test".to_string())
        .actions(vec![SudoAddressChange {
            new_address: *BOB_ADDRESS,
        }
        .into()])
        .try_build()
        .unwrap()
        .sign(&ALICE);

    let encoded_tx = Bytes::from(unchecked_tx.into_raw().encode_to_vec());
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            CheckedTransactionInitialCheckError::CheckedAction { .. }
        ),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_construction_if_chain_id_mismatch() {
    let fixture = Fixture::default_initialized().await;

    let unchecked_tx = TransactionBodyBuilder::new()
        .chain_id("wrong-chain".to_string())
        .actions(vec![SudoAddressChange {
            new_address: *BOB_ADDRESS,
        }
        .into()])
        .try_build()
        .unwrap()
        .sign(&SUDO);

    let encoded_tx = Bytes::from(unchecked_tx.into_raw().encode_to_vec());
    let error = CheckedTransaction::new(encoded_tx, fixture.state())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            CheckedTransactionInitialCheckError::ChainIdMismatch { .. }
        ),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_execution_if_nonce_invalid() {
    let mut fixture = Fixture::default_initialized().await;

    let tx = fixture
        .checked_tx_builder()
        .with_rollup_data_submission(vec![1, 2, 3])
        .with_signer(ALICE.clone())
        .with_nonce(10)
        .build()
        .await;

    let error = tx.execute(fixture.state_mut()).await.unwrap_err();
    assert!(
        matches!(error, CheckedTransactionExecutionError::InvalidNonce { .. }),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_fail_execution_if_action_fails_execution() {
    let mut fixture = Fixture::default_initialized().await;

    let tx = fixture
        .checked_tx_builder()
        .with_rollup_data_submission(vec![1, 2, 3])
        .with_signer(ALICE.clone())
        .build()
        .await;
    fixture
        .state_mut()
        .put_account_balance(&*ALICE_ADDRESS_BYTES, &nria(), 0)
        .unwrap();

    let error = tx.execute(fixture.state_mut()).await.unwrap_err();
    assert!(
        matches!(
            error,
            CheckedTransactionExecutionError::CheckedAction { .. }
        ),
        "{error:?}",
    );
}

#[tokio::test]
async fn should_execute_transfer() {
    let mut fixture = Fixture::default_initialized().await;

    // transfer funds from Alice to Bob
    let value = 333_333;
    let tx = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount: value,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    assert_eq!(
        fixture.get_nria_balance(&*BOB_ADDRESS).await,
        value + 10u128.pow(19)
    );
    let transfer_base = fixture
        .state()
        .get_fees::<Transfer>()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base();
    assert_eq!(
        fixture.get_nria_balance(&*ALICE_ADDRESS).await,
        10u128.pow(19) - (value + transfer_base),
    );
    assert_eq!(
        fixture
            .state()
            .get_account_nonce(&*BOB_ADDRESS)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        fixture
            .state()
            .get_account_nonce(&*ALICE_ADDRESS)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn should_execute_transfer_not_native_token() {
    let mut fixture = Fixture::default_initialized().await;

    // create some asset to be transferred and update Alice's balance of it
    let value = 333_333;

    fixture
        .state_mut()
        .put_account_balance(&*ALICE_ADDRESS, &test_asset(), value)
        .unwrap();

    // transfer funds from Alice to Bob; use native token for fee payment
    let tx = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount: value,
            asset: test_asset(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    assert_eq!(
        fixture.get_nria_balance(&*BOB_ADDRESS).await,
        10u128.pow(19), // genesis balance
    );
    assert_eq!(
        fixture
            .state()
            .get_account_balance(&*BOB_ADDRESS, &test_asset())
            .await
            .unwrap(),
        value, // transferred amount
    );

    let transfer_base = fixture
        .state()
        .get_fees::<Transfer>()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base();
    assert_eq!(
        fixture.get_nria_balance(&*ALICE_ADDRESS).await,
        10u128.pow(19) - transfer_base, // genesis balance - fee
    );
    assert_eq!(
        fixture
            .state()
            .get_account_balance(&*ALICE_ADDRESS, &test_asset())
            .await
            .unwrap(),
        0, // 0 since all funds of `asset` were transferred
    );

    assert_eq!(
        fixture
            .state()
            .get_account_nonce(&*BOB_ADDRESS)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        fixture
            .state()
            .get_account_nonce(&*ALICE_ADDRESS)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn execution_should_record_deposit_event() {
    let mut fixture = Fixture::default_initialized().await;
    fixture.bridge_initializer(*BOB_ADDRESS).init().await;

    let action = BridgeLock {
        to: *BOB_ADDRESS,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "test_chain_address".to_string(),
    };
    let tx = fixture
        .checked_tx_builder()
        .with_action(action)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let expected_deposit = Deposit {
        bridge_address: *BOB_ADDRESS,
        rollup_id: [1; 32].into(),
        amount: 1,
        asset: nria().into(),
        destination_chain_address: "test_chain_address".to_string(),
        source_transaction_id: *tx.id(),
        source_action_index: 0,
    };
    let expected_deposit_event = create_deposit_event(&expected_deposit);

    tx.execute(fixture.state_mut()).await.unwrap();
    let events = fixture.into_events();
    let event = events
        .iter()
        .find(|event| event.kind == "tx.deposit")
        .expect("should have deposit event");
    assert_eq!(*event, expected_deposit_event);
}

#[tokio::test]
async fn execution_should_record_fee_event() {
    let mut fixture = Fixture::default_initialized().await;

    // transfer funds from Alice to Bob
    let value = 333_333;
    let tx = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount: value,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();
    let events = fixture.into_events();

    let event = events.first().unwrap();
    assert_eq!(event.kind, "tx.fees");
    assert_eq!(event.attributes[0].key_bytes(), b"actionName");
    assert_eq!(event.attributes[1].key_bytes(), b"asset");
    assert_eq!(event.attributes[2].key_bytes(), b"feeAmount");
    assert_eq!(event.attributes[3].key_bytes(), b"positionInTransaction");
}
