use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::{
        BridgeUnlockAction,
        TransferAction,
    },
};
use tracing::instrument;

use crate::{
    accounts::action::transfer_check_stateful,
    bridge::state_ext::StateReadExt as _,
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeUnlockAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // the bridge address to withdraw funds from
        // if unset, use the tx sender's address
        let bridge_address = self.bridge_address.unwrap_or(from);

        // grab the bridge account's asset
        let asset_id = state
            .get_bridge_account_asset_id(&bridge_address)
            .await
            .context("failed to get bridge's asset id, must be a bridge account")?;

        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(&bridge_address)
            .await
            .context("failed to get bridge account withdrawer address")?
        else {
            bail!("bridge account does not have an associated withdrawer address");
        };

        ensure!(
            withdrawer_address == from,
            "unauthorized to unlock bridge account",
        );

        let transfer_action = TransferAction {
            to: self.to,
            asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        // this performs the same checks as a normal `TransferAction`
        transfer_check_stateful(&transfer_action, state, bridge_address).await
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        // the bridge address to withdraw funds from
        let bridge_address = self.bridge_address.unwrap_or(from);

        let asset_id = state
            .get_bridge_account_asset_id(&bridge_address)
            .await
            .context("failed to get bridge's asset id, must be a bridge account")?;

        let transfer_action = TransferAction {
            to: self.to,
            asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        transfer_action
            .execute(state, bridge_address)
            .await
            .context("failed to execute bridge unlock action as transfer action")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::primitive::v1::{
        asset,
        RollupId,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        bridge::state_ext::StateWriteExt,
        state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn bridge_unlock_fail_non_bridge_accounts() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_amount = 100;

        let address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: None,
        };

        // not a bridge account, should fail
        assert!(
            bridge_unlock
                .check_stateful(&state, address)
                .await
                .unwrap_err()
                .to_string()
                .contains("failed to get bridge's asset id, must be a bridge account")
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fail_withdrawer_unset_invalid_withdrawer() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_amount = 100;

        let sender_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);

        let bridge_address = Address::from([3; 20]);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_bridge_account_withdrawer_address(&bridge_address, &bridge_address);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: Some(bridge_address),
        };

        // invalid sender, doesn't match action's `from`, should fail
        assert!(
            bridge_unlock
                .check_stateful(&state, sender_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("unauthorized to unlock bridge account")
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fail_withdrawer_set_invalid_withdrawer() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_amount = 100;

        let sender_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);

        let bridge_address = Address::from([3; 20]);
        let withdrawer_address = Address::from([4; 20]);
        state.put_bridge_account_withdrawer_address(&bridge_address, &withdrawer_address);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: Some(bridge_address),
        };

        // invalid sender, doesn't match action's bridge account's withdrawer, should fail
        assert!(
            bridge_unlock
                .check_stateful(&state, sender_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("unauthorized to unlock bridge account")
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fee_check_stateful_from_none() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let bridge_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);
        state.put_bridge_account_withdrawer_address(&bridge_address, &bridge_address);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: None,
        };

        // not enough balance to transfer asset; should fail
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount)
            .unwrap();
        assert!(
            bridge_unlock
                .check_stateful(&state, bridge_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("insufficient funds for transfer and fee payment")
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock
            .check_stateful(&state, bridge_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bridge_unlock_fee_check_stateful_from_some() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let bridge_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);

        let withdrawer_address = Address::from([4; 20]);
        state.put_bridge_account_withdrawer_address(&bridge_address, &withdrawer_address);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: Some(bridge_address),
        };

        // not enough balance to transfer asset; should fail
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount)
            .unwrap();
        assert!(
            bridge_unlock
                .check_stateful(&state, withdrawer_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("insufficient funds for transfer and fee payment")
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock
            .check_stateful(&state, withdrawer_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bridge_unlock_execute_from_none() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let bridge_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: None,
        };

        // not enough balance; should fail
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount)
            .unwrap();
        assert!(
            bridge_unlock
                .execute(&mut state, bridge_address)
                .await
                .unwrap_err()
                .to_string()
                .eq("failed to execute bridge unlock action as transfer action")
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock
            .execute(&mut state, bridge_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bridge_unlock_execute_from_some() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = asset::Id::from_denom("test");
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let bridge_address = Address::from([1; 20]);
        let to_address = Address::from([2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_bridge_account_asset_id(&bridge_address, &asset_id)
            .unwrap();
        state.put_allowed_fee_asset(asset_id);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset_id: asset_id,
            memo: vec![0u8; 32],
            bridge_address: Some(bridge_address),
        };

        // not enough balance; should fail
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount)
            .unwrap();
        assert!(
            bridge_unlock
                .execute(&mut state, bridge_address)
                .await
                .unwrap_err()
                .to_string()
                .eq("failed to execute bridge unlock action as transfer action")
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, asset_id, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock
            .execute(&mut state, bridge_address)
            .await
            .unwrap();
    }
}
