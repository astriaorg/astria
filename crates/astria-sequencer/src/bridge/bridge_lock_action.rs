use astria_core::{
    primitive::v1::{
        Address,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
        ROLLUP_ID_LEN,
        TRANSACTION_ID_LEN,
    },
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

        let transaction_id = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .transaction_id;
        let source_action_index = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .source_action_index;

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset.clone(),
            self.destination_chain_address.clone(),
            transaction_id,
            source_action_index,
        );
        let deposit_abci_event = create_deposit_event(&deposit);

        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .wrap_err("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier
            .saturating_mul(get_deposit_byte_len(&deposit))
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
        let fee = byte_cost_multiplier.saturating_mul(get_deposit_byte_len(&deposit));
        state
            .get_and_increase_block_fees(&self.fee_asset, fee, Self::full_name())
            .await
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to deduct fee from account balance")?;

        state.record(deposit_abci_event);
        state
            .put_deposit_event(deposit)
            .await
            .wrap_err("failed to put deposit event into state")?;
        Ok(())
    }
}

/// Returns a modified byte length of the deposit event. Length is calculated with constants for all
/// fields except `asset` and `destination_chain_address`, ergo it may not be representative of
/// on-wire length. This should always return a byte length greater than or equal to the actual size
/// of the serialized `Deposit`, but never less.
pub(crate) fn get_deposit_byte_len(deposit: &Deposit) -> u128 {
    use prost::Message as _;
    let bridge_address = Address::builder()
        .prefix("astria")
        .slice(&[0; ADDRESS_LEN])
        .try_build()
        .unwrap();
    let deposit = Deposit::new(
        bridge_address,
        RollupId::from_unhashed_bytes([0; ROLLUP_ID_LEN]),
        u128::MAX,
        deposit.asset().clone(),
        deposit.destination_chain_address().to_string(),
        TransactionId::new([0; TRANSACTION_ID_LEN]),
        u64::MAX,
    );
    let raw = deposit.into_raw();
    raw.encoded_len() as u128
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset::{
            self,
        },
        RollupId,
        TransactionId,
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
            .put_account_balance(from_address, &asset, 100 + transfer_fee)
            .unwrap();
        assert_eyre_error(
            &bridge_lock.check_and_execute(&mut state).await.unwrap_err(),
            "insufficient funds for fee payment",
        );

        // enough balance; should pass
        let expected_deposit_fee = transfer_fee
            + get_deposit_byte_len(&Deposit::new(
                bridge_address,
                rollup_id,
                100,
                asset.clone(),
                "someaddress".to_string(),
                transaction_id,
                0,
            )) * 2;
        state
            .put_account_balance(from_address, &asset, 100 + expected_deposit_fee)
            .unwrap();
        bridge_lock.check_and_execute(&mut state).await.unwrap();
    }

    #[test]
    fn test_get_deposit_byte_len() {
        use prost::Message as _;

        // Test for deposit length at maximum int values
        let deposit = Deposit::new(
            astria_address(&[1; 20]),
            RollupId::from_unhashed_bytes(b"test_rollup_id"),
            u128::MAX,
            test_asset(),
            "someaddress".to_string(),
            TransactionId::new([0; 32]),
            u64::MAX,
        );
        let calculated_len = get_deposit_byte_len(&deposit);
        let expected_len = deposit.into_raw().encoded_len() as u128;
        assert_eq!(calculated_len, expected_len);

        // Test for deposit length at minimum int values
        let deposit = Deposit::new(
            astria_address(&[1; 20]),
            RollupId::from_unhashed_bytes(b"test_rollup_id"),
            0,
            test_asset(),
            "someaddress".to_string(),
            TransactionId::new([0; 32]),
            0,
        );
        let calculated_len = get_deposit_byte_len(&deposit);
        let expected_len = deposit.into_raw().encoded_len() as u128;
        assert!(calculated_len >= expected_len);

        // Ensure longer asset name results in longer byte length.
        let deposit = Deposit::new(
            astria_address(&[1; 20]),
            RollupId::from_unhashed_bytes(b"test_rollup_id"),
            0,
            "test_asset".parse().unwrap(),
            "someaddress".to_string(),
            TransactionId::new([0; 32]),
            0,
        );
        let calculated_len_2 = get_deposit_byte_len(&deposit);
        assert!(calculated_len_2 >= calculated_len);

        // Ensure longer destination chain address results in longer byte length.
        let deposit = Deposit::new(
            astria_address(&[1; 20]),
            RollupId::from_unhashed_bytes(b"test_rollup_id"),
            0,
            "test_asset".parse().unwrap(),
            "someaddresslonger".to_string(),
            TransactionId::new([0; 32]),
            0,
        );
        let calculated_len_3 = get_deposit_byte_len(&deposit);
        assert!(calculated_len_3 >= calculated_len_2);
    }
}
