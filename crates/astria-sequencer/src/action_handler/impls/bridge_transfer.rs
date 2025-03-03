use astria_core::protocol::transaction::v1::action::{
    BridgeTransfer,
    BridgeUnlock,
    Transfer,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::{
        check_transfer,
        create_deposit,
        execute_transfer,
        ActionHandler,
    },
    address::StateReadExt as _,
    assets::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

#[async_trait]
impl ActionHandler for BridgeTransfer {
    async fn check_stateless(&self) -> Result<()> {
        let bridge_unlock = BridgeUnlock {
            to: self.to,
            amount: self.amount,
            memo: String::new(),
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id.clone(),
            rollup_block_number: self.rollup_block_number,
            fee_asset: self.fee_asset.clone(),
            bridge_address: self.bridge_address,
        };
        bridge_unlock.check_stateless().await?;
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;

        // check that the assets for both bridge accounts match
        // also implicitly checks that both accounts are bridge accounts, as
        // only bridge accounts have an associated asset set
        let from_asset = state
            .get_bridge_account_ibc_asset(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge's asset id, must be a bridge account")?;
        let to_asset = state
            .get_bridge_account_ibc_asset(&self.to)
            .await
            .wrap_err("failed to get bridge's asset id, must be a bridge account")?;
        ensure!(
            from_asset == to_asset,
            "bridge accounts must have the same asset",
        );

        state
            .check_and_set_withdrawal_event_block_for_bridge_account(
                &self.bridge_address,
                &self.rollup_withdrawal_event_id,
                self.rollup_block_number,
            )
            .await
            .context("withdrawal event already processed")?;

        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        let bridge_asset = state
            .map_ibc_to_trace_prefixed_asset(&from_asset)
            .await
            .wrap_err("failed to map IBC asset to trace prefixed asset")?
            .ok_or_eyre("mapping from IBC prefixed bridge asset to trace prefixed not found")?;
        let deposit = create_deposit(&state, self, bridge_asset)
            .await
            .wrap_err("failed to construct deposit from state and bridge lock action")?;
        let deposit_abci_event = create_deposit_event(&deposit);

        let transfer_action = Transfer {
            to: self.to,
            asset: from_asset.into(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &from, &state).await?;
        execute_transfer(&transfer_action, &from, &mut state).await?;

        state.cache_deposit_event(deposit);
        state.record(deposit_abci_event);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            TransactionId,
        },
        protocol::transaction::v1::action::BridgeTransfer,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::{
            AddressBytes,
            StateWriteExt,
        },
        action_handler::impls::test_utils::test_asset,
        address::StateWriteExt as _,
        assets::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn bridge_transfer_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let from_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        state
            .put_bridge_account_ibc_asset(&from_address, &asset)
            .unwrap();
        state
            .put_bridge_account_withdrawer_address(&from_address, from_address)
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&to_address, &asset)
            .unwrap();
        let to_rollup_id = RollupId::new([3; 32]);
        state
            .put_bridge_account_rollup_id(&to_address, to_rollup_id)
            .unwrap();
        state
            .put_ibc_asset(test_asset().unwrap_trace_prefixed().clone())
            .unwrap();
        state
            .put_account_balance(&from_address, &asset, transfer_amount)
            .unwrap();

        let bridge_unlock = BridgeTransfer {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            bridge_address: from_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
            destination_chain_address: "noot".to_string(),
        };

        bridge_unlock.check_stateless().await.unwrap();
        bridge_unlock.check_and_execute(&mut state).await.unwrap();

        let deposits = state
            .get_cached_block_deposits()
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(deposits.len(), 1);
    }

    #[tokio::test]
    async fn bridge_transfer_accounts_have_different_asset_fails() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let from_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        state
            .put_bridge_account_ibc_asset(&from_address, &asset)
            .unwrap();
        state
            .put_bridge_account_withdrawer_address(&from_address, from_address)
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&to_address, "other-asset".parse::<Denom>().unwrap())
            .unwrap();
        let to_rollup_id = RollupId::new([3; 32]);
        state
            .put_bridge_account_rollup_id(&to_address, to_rollup_id)
            .unwrap();
        state
            .put_ibc_asset(test_asset().unwrap_trace_prefixed().clone())
            .unwrap();
        state
            .put_account_balance(&from_address, &asset, transfer_amount)
            .unwrap();

        let bridge_unlock = BridgeTransfer {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            bridge_address: from_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
            destination_chain_address: "noot".to_string(),
        };

        bridge_unlock.check_stateless().await.unwrap();
        let result = bridge_unlock.check_and_execute(state).await;
        assert_eyre_error(
            &result.unwrap_err(),
            "bridge accounts must have the same asset",
        );
    }
}
