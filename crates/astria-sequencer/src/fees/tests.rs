use std::sync::Arc;

use astria_core::{
    primitive::v1::{
        asset,
        Address,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
        ROLLUP_ID_LEN,
        TRANSACTION_ID_LEN,
    },
    protocol::{
        fees::v1::*,
        transaction::v1::{
            action::{
                BridgeLock,
                BridgeSudoChange,
                FeeChange,
                InitBridgeAccount,
                RollupDataSubmission,
                Transfer,
            },
            TransactionBody,
        },
    },
    sequencerblock::v1::block::Deposit,
};
use cnidarium::{
    Snapshot,
    StateDelta,
    StateWrite,
};

use super::base_deposit_fee;
use crate::{
    accounts::StateWriteExt as _,
    address::StateWriteExt as _,
    app::{
        test_utils::{
            get_alice_signing_key,
            get_bridge_signing_key,
            initialize_app_with_storage,
            BOB_ADDRESS,
        },
        ActionHandler as _,
    },
    authority::StateWriteExt as _,
    bridge::StateWriteExt as _,
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
        DEPOSIT_BASE_FEE,
    },
    test_utils::{
        assert_eyre_error,
        astria_address,
        astria_address_from_hex_string,
        calculate_rollup_data_submission_fee_from_state,
        nria,
        ASTRIA_PREFIX,
    },
    transaction::{
        StateWriteExt as _,
        TransactionContext,
    },
};

fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}

async fn setup_and_execute_fee_change<S: StateWrite>(state: &mut S, fee_change: FeeChange) {
    // Put the context to enable the txs to execute.
    state.put_transaction_context(TransactionContext {
        address_bytes: [1; 20],
        transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    });
    state.put_sudo_address([1; 20]).unwrap();

    fee_change.check_and_execute(state).await.unwrap();
}

async fn get_initial_state() -> StateDelta<Snapshot> {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    StateDelta::new(snapshot)
}

fn reference_deposit() -> Deposit {
    Deposit {
        bridge_address: astria_address(&[1; 20]),
        rollup_id: RollupId::from_unhashed_bytes(b"test_rollup_id"),
        amount: 0,
        asset: "test".parse().unwrap(),
        destination_chain_address: "someaddress".to_string(),
        source_transaction_id: TransactionId::new([0; 32]),
        source_action_index: 0,
    }
}

macro_rules! get_default_fees_and_fee_change {
    ($fee_ty:tt) => {
        paste::item! {
            {
                let initial_fees = [< $fee_ty FeeComponents >] {
                    base: 1,
                    multiplier: 2,
                };
                let initial_fee_change = FeeChange::$fee_ty(initial_fees);
                let new_fees = [< $fee_ty FeeComponents >] {
                    base: 3,
                    multiplier: 4,
                };
                let new_fee_change = FeeChange::$fee_ty(new_fees);
                (initial_fees, initial_fee_change, new_fees, new_fee_change)
            }
        }
    };
}

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

    let tx = TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
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
        .put_rollup_data_submission_fees(RollupDataSubmissionFeeComponents {
            base: 1,
            multiplier: 1,
        })
        .unwrap();

    let alice = get_alice_signing_key();
    let data = b"hello world".to_vec();

    let actions = vec![
        RollupDataSubmission {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: data.clone().into(),
            fee_asset: nria().into(),
        }
        .into(),
    ];

    let tx = TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();
    let total_block_fees: u128 = state
        .get_block_fees()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    let expected_fees = calculate_rollup_data_submission_fee_from_state(&data, &state).await;
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

    let tx = TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
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

    let tx = TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
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

    let tx = TransactionBody::builder()
        .actions(actions)
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&alice));
    signed_tx.check_and_execute(&mut state).await.unwrap();

    let total_block_fees: u128 = state
        .get_block_fees()
        .into_iter()
        .map(|fee| fee.amount())
        .sum();
    assert_eq!(total_block_fees, sudo_change_base);
}

#[tokio::test]
async fn bridge_lock_fee_calculation_works_as_expected() {
    let mut state = get_initial_state().await;
    let transfer_fee = 12;

    let from_address = astria_address(&[2; 20]);
    let transaction_id = TransactionId::new([0; 32]);
    state.put_transaction_context(TransactionContext {
        address_bytes: from_address.bytes(),
        transaction_id,
        source_action_index: 0,
    });
    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    let transfer_fees = TransferFeeComponents {
        base: transfer_fee,
        multiplier: 0,
    };
    state.put_transfer_fees(transfer_fees).unwrap();

    let bridge_lock_fees = BridgeLockFeeComponents {
        base: transfer_fee,
        multiplier: 2,
    };
    state.put_bridge_lock_fees(bridge_lock_fees).unwrap();

    let bridge_address = astria_address(&[1; 20]);
    let asset = test_asset();
    let bridge_lock = BridgeLock {
        to: bridge_address,
        asset: asset.clone(),
        amount: 100,
        fee_asset: asset.clone(),
        destination_chain_address: "someaddress".to_string(),
    };

    let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");
    state
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state
        .put_bridge_account_ibc_asset(&bridge_address, asset.clone())
        .unwrap();
    state.put_allowed_fee_asset(&asset).unwrap();

    // not enough balance; should fail
    state
        .put_account_balance(&from_address, &asset, transfer_fee)
        .unwrap();
    assert_eyre_error(
        &bridge_lock.check_and_execute(&mut state).await.unwrap_err(),
        "insufficient funds for transfer",
    );

    // enough balance; should pass
    let expected_deposit_fee = transfer_fee + base_deposit_fee(&asset, "someaddress") * 2;
    state
        .put_account_balance(&from_address, &asset, 100 + expected_deposit_fee)
        .unwrap();
    bridge_lock.check_and_execute(&mut state).await.unwrap();
}

#[test]
fn calculated_base_deposit_fee_matches_expected_value() {
    assert_correct_base_deposit_fee(&Deposit {
        amount: u128::MAX,
        source_action_index: u64::MAX,
        ..reference_deposit()
    });
    assert_correct_base_deposit_fee(&Deposit {
        asset: "test_asset".parse().unwrap(),
        ..reference_deposit()
    });
    assert_correct_base_deposit_fee(&Deposit {
        destination_chain_address: "someaddresslonger".to_string(),
        ..reference_deposit()
    });

    // Ensure calculated length is as expected with absurd string
    // lengths (have tested up to 99999999, but this makes testing very slow)
    let absurd_string: String = ['a'; u16::MAX as usize].iter().collect();
    assert_correct_base_deposit_fee(&Deposit {
        asset: absurd_string.parse().unwrap(),
        ..reference_deposit()
    });
    assert_correct_base_deposit_fee(&Deposit {
        destination_chain_address: absurd_string,
        ..reference_deposit()
    });
}

#[track_caller]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "adding length of strings will never overflow u128 on currently existing machines"
)]
fn assert_correct_base_deposit_fee(deposit: &Deposit) {
    let calculated_len = base_deposit_fee(&deposit.asset, &deposit.destination_chain_address);
    let expected_len = DEPOSIT_BASE_FEE
        + deposit.asset.to_string().len() as u128
        + deposit.destination_chain_address.len() as u128;
    assert_eq!(calculated_len, expected_len);
}

/// Used to determine the base deposit byte length for `get_deposit_byte_len()`. This is based
/// on "reasonable" values for all fields except `asset` and `destination_chain_address`. These
/// are empty strings, whose length will be added to the base cost at the time of
/// calculation.
///
/// This test determines 165 bytes for an average deposit with empty `asset` and
/// `destination_chain_address`, which is divided by 10 to get our base byte length of 16. This
/// is to allow for more flexibility in overall fees (we have more flexibility multiplying by a
/// lower number, and if we want fees to be higher we can just raise the multiplier).
#[test]
fn get_base_deposit_fee() {
    use prost::Message as _;
    let bridge_address = Address::builder()
        .prefix("astria-bridge")
        .slice(&[0u8; ADDRESS_LEN][..])
        .try_build()
        .unwrap();
    let raw_deposit = astria_core::generated::sequencerblock::v1::Deposit {
        bridge_address: Some(bridge_address.to_raw()),
        rollup_id: Some(RollupId::from_unhashed_bytes([0; ROLLUP_ID_LEN]).to_raw()),
        amount: Some(1000.into()),
        asset: String::new(),
        destination_chain_address: String::new(),
        source_transaction_id: Some(TransactionId::new([0; TRANSACTION_ID_LEN]).to_raw()),
        source_action_index: 0,
    };
    assert_eq!(DEPOSIT_BASE_FEE, raw_deposit.encoded_len() as u128 / 10);
}

#[tokio::test]
async fn transfer_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_transfer_fees()
            .await
            .expect("should not error fetching transfer fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(Transfer);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_transfer_fees()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_transfer_fees()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn rollup_data_submission_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_rollup_data_submission_fees()
            .await
            .expect("should not error fetching transfer fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(RollupDataSubmission);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_rollup_data_submission_fees()
        .await
        .expect("should not error fetching rollup data submission fees")
        .expect("rollup data submission fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_rollup_data_submission_fees()
        .await
        .expect("should not error fetching rollup data submission fees")
        .expect("rollup data submission fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn init_bridge_account_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_init_bridge_account_fees()
            .await
            .expect("should not error fetching transfer fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(InitBridgeAccount);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_init_bridge_account_fees()
        .await
        .expect("should not error fetching init_bridge_account fees")
        .expect("init_bridge_account fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_init_bridge_account_fees()
        .await
        .expect("should not error fetching init_bridge_account fees")
        .expect("init_bridge_account fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn bridge_lock_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_bridge_lock_fees()
            .await
            .expect("should not error fetching bridge_lock fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(BridgeLock);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_bridge_lock_fees()
        .await
        .expect("should not error fetching bridge_lock fees")
        .expect("bridge_lock fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_bridge_lock_fees()
        .await
        .expect("should not error fetching bridge_lock fees")
        .expect("bridge_lock fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn bridge_unlock_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_bridge_unlock_fees()
            .await
            .expect("should not error fetching bridge_unlock fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(BridgeUnlock);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_bridge_unlock_fees()
        .await
        .expect("should not error fetching bridge_unlock fees")
        .expect("bridge_unlock fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_bridge_unlock_fees()
        .await
        .expect("should not error fetching bridge_unlock fees")
        .expect("bridge_unlock fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn bridge_sudo_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_bridge_sudo_change_fees()
            .await
            .expect("should not error fetching bridge_sudo_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(BridgeSudoChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_bridge_sudo_change_fees()
        .await
        .expect("should not error fetching bridge_sudo_change fees")
        .expect("bridge_sudo_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_bridge_sudo_change_fees()
        .await
        .expect("should not error fetching bridge_sudo_change fees")
        .expect("bridge_sudo_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn validator_update_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_validator_update_fees()
            .await
            .expect("should not error fetching validator_update fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(ValidatorUpdate);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_validator_update_fees()
        .await
        .expect("should not error fetching validator_update fees")
        .expect("validator_update fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_validator_update_fees()
        .await
        .expect("should not error fetching validator_update fees")
        .expect("validator_update fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn ibc_relayer_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_ibc_relayer_change_fees()
            .await
            .expect("should not error fetching ibc_relayer_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(IbcRelayerChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_ibc_relayer_change_fees()
        .await
        .expect("should not error fetching ibc_relayer_change fees")
        .expect("ibc_relayer_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_ibc_relayer_change_fees()
        .await
        .expect("should not error fetching ibc_relayer_change fees")
        .expect("ibc_relayer_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn ibc_relay_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_ibc_relay_fees()
            .await
            .expect("should not error fetching ibc_relay fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(IbcRelay);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_ibc_relay_fees()
        .await
        .expect("should not error fetching ibc_relay fees")
        .expect("ibc_relay fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_ibc_relay_fees()
        .await
        .expect("should not error fetching ibc_relay fees")
        .expect("ibc_relay fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn fee_asset_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_fee_asset_change_fees()
            .await
            .expect("should not error fetching fee_asset_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(FeeAssetChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_fee_asset_change_fees()
        .await
        .expect("should not error fetching fee_asset_change fees")
        .expect("fee_asset_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_fee_asset_change_fees()
        .await
        .expect("should not error fetching fee_asset_change fees")
        .expect("fee_asset_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn fee_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_fee_change_fees()
            .await
            .expect("should not error fetching fee_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(FeeChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_fee_change_fees()
        .await
        .expect("should not error fetching fee_change fees")
        .expect("fee_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_fee_change_fees()
        .await
        .expect("should not error fetching fee_change fees")
        .expect("fee_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn sudo_address_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_sudo_address_change_fees()
            .await
            .expect("should not error fetching sudo_address_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(SudoAddressChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_sudo_address_change_fees()
        .await
        .expect("should not error fetching sudo_address_change fees")
        .expect("sudo_address_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_sudo_address_change_fees()
        .await
        .expect("should not error fetching sudo_address_change fees")
        .expect("sudo_address_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

#[tokio::test]
async fn ibc_sudo_change_fee_change_executes_as_expected() {
    let mut state = get_initial_state().await;

    // Ensure fees are not stored initially
    assert!(
        state
            .get_ibc_sudo_change_fees()
            .await
            .expect("should not error fetching ibc_sudo_change fees")
            .is_none()
    );

    // Generate initial and new fees, along with corresponding fee change actions
    let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
        get_default_fees_and_fee_change!(IbcSudoChange);

    // Execute and check fee initial fee change action
    setup_and_execute_fee_change(&mut state, initial_fee_change).await;
    let retrieved_fees = state
        .get_ibc_sudo_change_fees()
        .await
        .expect("should not error fetching ibc_sudo_change fees")
        .expect("ibc_sudo_change fees should be stored");
    assert_eq!(initial_fees, retrieved_fees);

    // Execute and check new fee change action
    setup_and_execute_fee_change(&mut state, new_fee_change).await;
    let retrieved_fees = state
        .get_ibc_sudo_change_fees()
        .await
        .expect("should not error fetching ibc_sudo_change fees")
        .expect("ibc_sudo_change fees should be stored");
    assert_eq!(new_fees, retrieved_fees);
}

// TODO(https://github.com/astriaorg/astria/issues/1382): Add test to ensure correct block fees for ICS20 withdrawal
