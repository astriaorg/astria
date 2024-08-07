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
use cnidarium::StateWrite;

use crate::{
    accounts::action::{
        check_transfer,
        execute_transfer,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    bridge::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeUnlockAction {
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .context("failed check for base prefix of destination address")?;
        if let Some(bridge_address) = &self.bridge_address {
            state
                .ensure_base_prefix(bridge_address)
                .await
                .context("failed check for base prefix of bridge address")?;
        }

        // the bridge address to withdraw funds from
        // if unset, use the tx sender's address
        let bridge_address = self.bridge_address.map_or(from, Address::bytes);

        let asset = state
            .get_bridge_account_ibc_asset(bridge_address)
            .await
            .context("failed to get bridge's asset id, must be a bridge account")?;

        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(bridge_address)
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
            asset: asset.into(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, bridge_address, &state).await?;
        execute_transfer(&transfer_action, bridge_address, state).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset,
            RollupId,
        },
        protocol::transaction::v1alpha1::action::BridgeUnlockAction,
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::StateWriteExt as _,
        address::StateWriteExt as _,
        app::ActionHandler as _,
        assets::StateWriteExt as _,
        bridge::StateWriteExt as _,
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
    async fn fails_if_bridge_address_is_not_set_and_signer_is_not_bridge() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_current_source(TransactionContext {
            address_bytes: [1; 20],
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset,
            memo: "{}".into(),
            bridge_address: None,
        };

        // not a bridge account, should fail
        assert!(
            bridge_unlock
                .check_and_execute(state)
                .await
                .unwrap_err()
                .to_string()
                .contains("failed to get bridge's asset id, must be a bridge account")
        );
    }

    #[tokio::test]
    async fn fails_if_bridge_account_has_no_withdrawer_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_current_source(TransactionContext {
            address_bytes: [1; 20],
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        let bridge_address = astria_address(&[3; 20]);
        // state.put_bridge_account_withdrawer_address(bridge_address, bridge_address);
        state
            .put_bridge_account_ibc_asset(bridge_address, &asset)
            .unwrap();

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            memo: "{}".into(),
            bridge_address: Some(bridge_address),
        };

        // invalid sender, doesn't match action's `from`, should fail
        assert_anyhow_error(
            &bridge_unlock.check_and_execute(state).await.unwrap_err(),
            "bridge account does not have an associated withdrawer address",
        );
    }

    #[tokio::test]
    async fn fails_if_withdrawer_is_not_signer() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_current_source(TransactionContext {
            address_bytes: [1; 20],
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        let bridge_address = astria_address(&[3; 20]);
        let withdrawer_address = astria_address(&[4; 20]);
        state.put_bridge_account_withdrawer_address(bridge_address, withdrawer_address);
        state
            .put_bridge_account_ibc_asset(bridge_address, &asset)
            .unwrap();

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset,
            memo: "{}".into(),
            bridge_address: Some(bridge_address),
        };

        // invalid sender, doesn't match action's bridge account's withdrawer, should fail
        assert_anyhow_error(
            &bridge_unlock.check_and_execute(state).await.unwrap_err(),
            "unauthorized to unlock bridge account",
        );
    }

    #[tokio::test]
    async fn execute_with_bridge_address_unset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = astria_address(&[1; 20]);
        state.put_current_source(TransactionContext {
            address_bytes: bridge_address.bytes(),
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset = test_asset();
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let to_address = astria_address(&[2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state
            .put_bridge_account_ibc_asset(bridge_address, &asset)
            .unwrap();
        state.put_bridge_account_withdrawer_address(bridge_address, bridge_address);
        state.put_allowed_fee_asset(&asset);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            memo: "{}".into(),
            bridge_address: None,
        };

        // not enough balance; should fail
        state
            .put_account_balance(bridge_address, &asset, transfer_amount)
            .unwrap();
        assert_anyhow_error(
            &bridge_unlock
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "insufficient funds for transfer and fee payment",
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, &asset, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock.check_and_execute(&mut state).await.unwrap();
    }

    #[tokio::test]
    async fn execute_with_bridge_address_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = astria_address(&[1; 20]);
        state.put_current_source(TransactionContext {
            address_bytes: bridge_address.bytes(),
        });
        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset = test_asset();
        let transfer_fee = 10;
        let transfer_amount = 100;
        state.put_transfer_base_fee(transfer_fee).unwrap();

        let to_address = astria_address(&[2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state
            .put_bridge_account_ibc_asset(bridge_address, &asset)
            .unwrap();
        state.put_bridge_account_withdrawer_address(bridge_address, bridge_address);
        state.put_allowed_fee_asset(&asset);

        let bridge_unlock = BridgeUnlockAction {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            memo: "{}".into(),
            bridge_address: Some(bridge_address),
        };

        // not enough balance; should fail
        state
            .put_account_balance(bridge_address, &asset, transfer_amount)
            .unwrap();
        assert_anyhow_error(
            &bridge_unlock
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "insufficient funds for transfer and fee payment",
        );

        // enough balance; should pass
        state
            .put_account_balance(bridge_address, &asset, transfer_amount + transfer_fee)
            .unwrap();
        bridge_unlock.check_and_execute(&mut state).await.unwrap();
    }
}
