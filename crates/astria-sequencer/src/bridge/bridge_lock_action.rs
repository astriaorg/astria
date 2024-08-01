use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    protocol::transaction::v1alpha1::action::{
        BridgeLockAction,
        TransferAction,
    },
    sequencerblock::v1alpha1::block::Deposit,
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
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeLockAction {
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .context("failed check for base prefix of destination address")?;
        // ensure the recipient is a bridge account.
        let rollup_id = state
            .get_bridge_account_rollup_id(self.to)
            .await
            .context("failed to get bridge account rollup id")?
            .ok_or_else(|| anyhow::anyhow!("bridge lock must be sent to a bridge account"))?;

        let allowed_asset = state
            .get_bridge_account_ibc_asset(self.to)
            .await
            .context("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        let from_balance = state
            .get_account_balance(from, &self.fee_asset)
            .await
            .context("failed to get sender account balance")?;
        let transfer_fee = state
            .get_transfer_base_fee()
            .await
            .context("failed to get transfer base fee")?;

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset.clone(),
            self.destination_chain_address.clone(),
        );

        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .context("failed to get byte cost multiplier")?;
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
        execute_transfer(&transfer_action, from, &mut state).await?;

        let rollup_id = state
            .get_bridge_account_rollup_id(self.to)
            .await
            .context("failed to get bridge account rollup id")?
            .expect("recipient must be a bridge account; this is a bug in check_stateful");

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset.clone(),
            self.destination_chain_address.clone(),
        );

        // the transfer fee is already deducted in `transfer_action.execute()`,
        // so we just deduct the bridge lock byte multiplier fee.
        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .context("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier.saturating_mul(get_deposit_byte_len(&deposit));

        state
            .decrease_balance(from, &self.fee_asset, fee)
            .await
            .context("failed to deduct fee from account balance")?;

        state
            .put_deposit_event(deposit)
            .await
            .context("failed to put deposit event into state")?;
        Ok(())
    }
}

/// returns the length of a serialized `Deposit` message.
pub(crate) fn get_deposit_byte_len(deposit: &Deposit) -> u128 {
    use prost::Message as _;
    let raw = deposit.clone().into_raw();
    raw.encoded_len() as u128
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset,
        RollupId,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        address::StateWriteExt,
        assets::StateWriteExt as _,
        test_utils::{
            assert_anyhow_error,
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
        state.put_current_source(TransactionContext {
            address_bytes: from_address.bytes(),
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

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
        assert_anyhow_error(
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
            )) * 2;
        state
            .put_account_balance(from_address, &asset, 100 + expected_deposit_fee)
            .unwrap();
        bridge_lock.check_and_execute(&mut state).await.unwrap();
    }
}
