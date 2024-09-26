use astria_core::{
    protocol::transaction::v1alpha1::action::{
        BridgeLockAction,
        TransferAction,
    },
    sequencerblock::v1alpha1::block::Deposit,
    Protobuf as _,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    accounts::{
        action::{
            check_transfer,
            execute_transfer,
        },
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    assets::StateWriteExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
const DEPOSIT_BASE_FEE: u128 = 16;

#[async_trait::async_trait]
impl ActionHandler for BridgeLockAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        // ensure the recipient is a bridge account.
        let rollup_id = state
            .get_bridge_account_rollup_id(self.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;

        let allowed_asset = state
            .get_bridge_account_ibc_asset(self.to)
            .await
            .wrap_err("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        let from_balance = state
            .get_account_balance(from, &self.fee_asset)
            .await
            .wrap_err("failed to get sender account balance")?;
        let transfer_fee = state
            .get_transfer_base_fee()
            .await
            .context("failed to get transfer base fee")?;

        let source_transaction_id = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .transaction_id;
        let source_action_index = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .source_action_index;

        let deposit = Deposit {
            bridge_address: self.to,
            rollup_id,
            amount: self.amount,
            asset: self.asset.clone(),
            destination_chain_address: self.destination_chain_address.clone(),
            source_transaction_id,
            source_action_index,
        };
        let deposit_abci_event = create_deposit_event(&deposit);

        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .wrap_err("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier
            .saturating_mul(calculate_base_deposit_fee(&deposit).unwrap_or(u128::MAX))
            .saturating_add(transfer_fee);
        ensure!(from_balance >= fee, "insufficient funds for fee payment");

        let transfer_action = TransferAction {
            to: self.to,
            asset: self.asset.clone(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, from, &state).await?;
        // Executes the transfer and deducts transfer feeds.
        // FIXME: This is a very roundabout way of paying for fees. IMO it would be
        // better to just duplicate this entire logic here so that we don't call out
        // to the transfer-action logic.
        execute_transfer(&transfer_action, from, &mut state).await?;

        // the transfer fee is already deducted in `execute_transfer() above,
        // so we just deduct the bridge lock byte multiplier fee.
        // FIXME: similar to what is mentioned there: this should be reworked so that
        // the fee deducation logic for these actions are defined fully independently
        // (even at the cost of duplicating code).
        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .wrap_err("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier
            .saturating_mul(calculate_base_deposit_fee(&deposit).unwrap_or(u128::MAX));
        state
            .get_and_increase_block_fees(&self.fee_asset, fee, Self::full_name())
            .await
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to deduct fee from account balance")?;

        state.record(deposit_abci_event);
        state.cache_deposit_event(deposit);
        Ok(())
    }
}

/// Returns a modified byte length of the deposit event. Length is calculated with reasonable values
/// for all fields except `asset` and `destination_chain_address`, ergo it may not be representative
/// of on-wire length.
pub(crate) fn calculate_base_deposit_fee(deposit: &Deposit) -> Option<u128> {
    deposit
        .asset
        .display_len()
        .checked_add(deposit.destination_chain_address.len())
        .and_then(|var_len| {
            DEPOSIT_BASE_FEE.checked_add(u128::try_from(var_len).expect(
                "converting a usize to a u128 should work on any currently existing machine",
            ))
        })
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset::{
            self,
        },
        Address,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
        ROLLUP_ID_LEN,
        TRANSACTION_ID_LEN,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        test_utils::{
            assert_eyre_error,
            astria_address,
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

    #[tokio::test]
    async fn execute_fee_calc() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;

        let from_address = astria_address(&[2; 20]);
        let transaction_id = TransactionId::new([0; 32]);
        state.put_transaction_context(TransactionContext {
            address_bytes: from_address.bytes(),
            transaction_id,
            source_action_index: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX);

        state.put_transfer_base_fee(transfer_fee).unwrap();
        state.put_bridge_lock_byte_cost_multiplier(2);

        let bridge_address = astria_address(&[1; 20]);
        let asset = test_asset();
        let bridge_lock = BridgeLockAction {
            to: bridge_address,
            asset: asset.clone(),
            amount: 100,
            fee_asset: asset.clone(),
            destination_chain_address: "someaddress".to_string(),
        };

        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");
        state.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state
            .put_bridge_account_ibc_asset(bridge_address, &asset)
            .unwrap();
        state.put_allowed_fee_asset(&asset);

        // not enough balance; should fail
        state
            .put_account_balance(from_address, &asset, transfer_fee)
            .unwrap();
        assert_eyre_error(
            &bridge_lock.check_and_execute(&mut state).await.unwrap_err(),
            "insufficient funds for fee payment",
        );

        // enough balance; should pass
        let expected_deposit_fee = transfer_fee
            + calculate_base_deposit_fee(&Deposit {
                bridge_address,
                rollup_id,
                amount: 100,
                asset: asset.clone(),
                destination_chain_address: "someaddress".to_string(),
                source_transaction_id: transaction_id,
                source_action_index: 0,
            })
            .unwrap()
                * 2;
        state
            .put_account_balance(from_address, &asset, 100 + expected_deposit_fee)
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
    #[allow(clippy::arithmetic_side_effects)] // allow: test will never overflow u128
    fn assert_correct_base_deposit_fee(deposit: &Deposit) {
        let calculated_len = calculate_base_deposit_fee(deposit).unwrap();
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
        let raw_deposit = astria_core::generated::sequencerblock::v1alpha1::Deposit {
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
}
