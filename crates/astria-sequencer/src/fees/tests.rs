use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset,
        Address,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
        ROLLUP_ID_LEN,
        TRANSACTION_ID_LEN,
    },
    protocol::transaction::v1::{
        action::{
            BridgeLock,
            BridgeSudoChange,
            InitBridgeAccount,
            RollupDataSubmission,
            Transfer,
        },
        Action,
    },
    sequencerblock::v1::block::Deposit,
    Protobuf,
};
use astria_eyre::eyre::WrapErr as _;
use cnidarium::StateDelta;
use prost::Name as _;

use super::{
    fee_handler::base_deposit_fee,
    FeeHandler,
    StateWriteExt as _,
};
use crate::{
    accounts::StateWriteExt as _,
    fees::{
        fee_handler::DEPOSIT_BASE_FEE,
        StateReadExt as _,
    },
    test_utils::{
        assert_error_contains,
        astria_address,
        nria,
        Fixture,
        ALICE,
        BOB_ADDRESS,
        IBC_SUDO_ADDRESS,
        SUDO_ADDRESS_BYTES,
    },
};

fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}

fn total_block_fees(fixture: &Fixture) -> u128 {
    fixture.state().get_block_fees().values().sum()
}

#[tokio::test]
async fn ensure_correct_block_fees_transfer() {
    let mut fixture = Fixture::default_initialized().await;
    let transfer_base = fixture.genesis_app_state().fees().transfer.unwrap().base();

    let tx = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount: 1000,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    assert_eq!(total_block_fees(&fixture), transfer_base);
}

#[tokio::test]
async fn ensure_correct_block_fees_sequence() {
    let mut fixture = Fixture::default_initialized().await;

    let data = b"hello world".to_vec();
    let tx = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: data.clone().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    let expected_fees = fixture.calculate_rollup_data_submission_cost(&data).await;
    assert_eq!(total_block_fees(&fixture), expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_init_bridge_acct() {
    let mut fixture = Fixture::default_initialized().await;
    let init_bridge_account_base = fixture
        .genesis_app_state()
        .fees()
        .init_bridge_account
        .unwrap()
        .base();

    let tx = fixture
        .checked_tx_builder()
        .with_action(InitBridgeAccount {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    assert_eq!(total_block_fees(&fixture), init_bridge_account_base);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_lock() {
    let mut fixture = Fixture::default_initialized().await;
    let bridge_lock_base = fixture
        .genesis_app_state()
        .fees()
        .bridge_lock
        .unwrap()
        .base();
    let bridge_lock_byte_cost_multiplier = fixture
        .genesis_app_state()
        .fees()
        .bridge_lock
        .unwrap()
        .multiplier();

    fixture.bridge_initializer(*IBC_SUDO_ADDRESS).init().await;

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let tx = fixture
        .checked_tx_builder()
        .with_action(BridgeLock {
            to: *IBC_SUDO_ADDRESS,
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: rollup_id.to_string(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    let expected_fees = bridge_lock_base
        + (base_deposit_fee(&nria().into(), rollup_id.to_string().as_str())
            * bridge_lock_byte_cost_multiplier);
    assert_eq!(total_block_fees(&fixture), expected_fees);
}

#[tokio::test]
async fn ensure_correct_block_fees_bridge_sudo_change() {
    let mut fixture = Fixture::default_initialized().await;
    let sudo_change_base = fixture
        .genesis_app_state()
        .fees()
        .bridge_sudo_change
        .unwrap()
        .base();

    fixture.bridge_initializer(*IBC_SUDO_ADDRESS).init().await;
    fixture
        .state_mut()
        .put_account_balance(&*SUDO_ADDRESS_BYTES, &nria(), 999)
        .unwrap();

    let tx = fixture
        .checked_tx_builder()
        .with_action(BridgeSudoChange {
            bridge_address: *IBC_SUDO_ADDRESS,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: nria().into(),
            disable_deposits: false,
        })
        .build()
        .await;
    tx.execute(fixture.state_mut()).await.unwrap();

    assert_eq!(total_block_fees(&fixture), sudo_change_base);
}

#[tokio::test]
async fn bridge_lock_fee_calculation_works_as_expected() {
    let mut fixture = Fixture::default_initialized().await;
    let bridge_lock_base = fixture
        .genesis_app_state()
        .fees()
        .bridge_lock
        .unwrap()
        .base();
    let bridge_lock_multiplier = fixture
        .genesis_app_state()
        .fees()
        .bridge_lock
        .unwrap()
        .multiplier();

    let bridge_address = astria_address(&[1; 20]);
    fixture
        .bridge_initializer(bridge_address)
        .with_asset(test_asset())
        .init()
        .await;
    fixture
        .state_mut()
        .put_allowed_fee_asset(&test_asset())
        .unwrap();

    let signer = SigningKey::from([10; 32]);
    let lock_amount = 100;

    let tx = fixture
        .checked_tx_builder()
        .with_action(BridgeLock {
            to: bridge_address,
            asset: test_asset(),
            amount: lock_amount,
            fee_asset: test_asset(),
            destination_chain_address: "someaddress".to_string(),
        })
        .with_signer(signer.clone())
        .build()
        .await;

    // not enough balance; should fail
    let mut state_delta = StateDelta::new(fixture.state_mut());
    state_delta
        .put_account_balance(&signer.verification_key(), &test_asset(), bridge_lock_base)
        .unwrap();
    let error = format!(
        "{:#}",
        tx.execute(state_delta)
            .await
            .wrap_err("failed execution")
            .unwrap_err()
    );
    assert_error_contains(
        &error,
        &format!("insufficient {} balance in account", test_asset()),
    );

    // enough balance; should pass
    let expected_deposit_fee =
        bridge_lock_base + base_deposit_fee(&test_asset(), "someaddress") * bridge_lock_multiplier;
    fixture
        .state_mut()
        .put_account_balance(
            &signer.verification_key(),
            &test_asset(),
            lock_amount + expected_deposit_fee,
        )
        .unwrap();
    tx.execute(fixture.state_mut()).await.unwrap();
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
    #[expect(clippy::large_stack_arrays, reason = "test only")]
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
    let raw_deposit = astria_core::generated::astria::sequencerblock::v1::Deposit {
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

#[test]
fn ensure_fee_handler_consts_valid() {
    fn check_names<F: FeeHandler + Protobuf>(action: &F) {
        assert_eq!(action.name(), F::Raw::NAME);
        assert_eq!(<F as FeeHandler>::full_name(), <F as Protobuf>::full_name());
        let mut chars_iter = action.name().chars();
        let mut snake_case_name = String::from(chars_iter.next().unwrap().to_ascii_lowercase());
        for char in chars_iter {
            if char.is_ascii_uppercase() {
                snake_case_name.push('_');
            }
            snake_case_name.push(char.to_ascii_lowercase());
        }
        assert_eq!(F::snake_case_name(), snake_case_name);
    }
    for action in &crate::checked_actions::test_utils::dummy_actions() {
        match action {
            Action::RollupDataSubmission(action) => check_names(action),
            Action::Transfer(action) => check_names(action),
            Action::ValidatorUpdate(action) => check_names(action),
            Action::SudoAddressChange(action) => check_names(action),
            Action::Ibc(_action) => (), // check_names(action),
            Action::IbcSudoChange(action) => check_names(action),
            Action::Ics20Withdrawal(action) => check_names(action),
            Action::IbcRelayerChange(action) => check_names(action),
            Action::FeeAssetChange(action) => check_names(action),
            Action::InitBridgeAccount(action) => check_names(action),
            Action::BridgeLock(action) => check_names(action),
            Action::BridgeUnlock(action) => check_names(action),
            Action::BridgeSudoChange(action) => check_names(action),
            Action::BridgeTransfer(action) => check_names(action),
            Action::FeeChange(action) => check_names(action),
            Action::RecoverIbcClient(action) => check_names(action),
            Action::CurrencyPairsChange(action) => check_names(action),
            Action::MarketsChange(action) => check_names(action),
        }
    }
}

// TODO(https://github.com/astriaorg/astria/issues/1382): Add test to ensure correct block fees for ICS20 withdrawal
