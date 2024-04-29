use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::{
        BridgeLockAction,
        TransferAction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use tracing::instrument;

use crate::{
    accounts::{
        action::transfer_check_stateful,
        state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeLockAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        let transfer_action = TransferAction {
            to: self.to,
            asset_id: self.asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        // ensure the recipient is a bridge account.
        let rollup_id = state
            .get_bridge_account_rollup_id(&self.to)
            .await
            .context("failed to get bridge account rollup id")?
            .ok_or_else(|| anyhow::anyhow!("bridge lock must be sent to a bridge account"))?;

        let allowed_asset_id = state
            .get_bridge_account_asset_ids(&self.to)
            .await
            .context("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset_id == self.asset_id,
            "asset ID is not authorized for transfer to bridge account",
        );

        let from_balance = state
            .get_account_balance(from, self.fee_asset_id)
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
            self.asset_id,
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

        // this performs the same checks as a normal `TransferAction`,
        // but without the check that prevents transferring to a bridge account,
        // as we are explicitly transferring to a bridge account here.
        transfer_check_stateful(&transfer_action, state, from).await
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let transfer_action = TransferAction {
            to: self.to,
            asset_id: self.asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        transfer_action
            .execute(state, from)
            .await
            .context("failed to execute bridge lock action as transfer action")?;

        let rollup_id = state
            .get_bridge_account_rollup_id(&self.to)
            .await
            .context("failed to get bridge account rollup id")?
            .expect("recipient must be a bridge account; this is a bug in check_stateful");

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset_id,
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
            .decrease_balance(from, self.fee_asset_id, fee)
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
mod test {
    use astria_core::primitive::v1::{
        asset,
        RollupId,
    };
    use cnidarium::StateDelta;

    use super::*;

    #[tokio::test]
    async fn bridge_lock_check_stateful_fee_calc() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;
        state.put_transfer_base_fee(transfer_fee).unwrap();
        state.put_bridge_lock_byte_cost_multiplier(2);

        let bridge_address = Address::from([1; 20]);
        let asset_id = asset::Id::from_denom("test");
        let bridge_lock = BridgeLockAction {
            to: bridge_address,
            asset_id,
            amount: 100,
            fee_asset_id: asset_id,
            destination_chain_address: "someaddress".to_string(),
        };

        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");
        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);

        let from_address = Address::from([2; 20]);

        // not enough balance; should fail
        state
            .put_account_balance(from_address, asset_id, 100)
            .unwrap();
        assert!(
            bridge_lock
                .check_stateful(&state, from_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("insufficient funds for fee payment")
        );

        // enough balance; should pass
        let expected_deposit_fee = transfer_fee
            + get_deposit_byte_len(&Deposit::new(
                bridge_address,
                rollup_id,
                100,
                asset_id,
                "someaddress".to_string(),
            )) * 2;
        state
            .put_account_balance(from_address, asset_id, 100 + expected_deposit_fee)
            .unwrap();
        bridge_lock
            .check_stateful(&state, from_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bridge_lock_execute_fee_calc() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;
        state.put_transfer_base_fee(transfer_fee).unwrap();
        state.put_bridge_lock_byte_cost_multiplier(2);

        let bridge_address = Address::from([1; 20]);
        let asset_id = asset::Id::from_denom("test");
        let bridge_lock = BridgeLockAction {
            to: bridge_address,
            asset_id,
            amount: 100,
            fee_asset_id: asset_id,
            destination_chain_address: "someaddress".to_string(),
        };

        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");
        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);

        let from_address = Address::from([2; 20]);

        // not enough balance; should fail
        state
            .put_account_balance(from_address, asset_id, 100 + transfer_fee)
            .unwrap();
        assert!(
            bridge_lock
                .execute(&mut state, from_address)
                .await
                .unwrap_err()
                .to_string()
                .eq("failed to deduct fee from account balance")
        );

        // enough balance; should pass
        let expected_deposit_fee = transfer_fee
            + get_deposit_byte_len(&Deposit::new(
                bridge_address,
                rollup_id,
                100,
                asset_id,
                "someaddress".to_string(),
            )) * 2;
        state
            .put_account_balance(from_address, asset_id, 100 + expected_deposit_fee)
            .unwrap();
        bridge_lock.execute(&mut state, from_address).await.unwrap();
    }
}
