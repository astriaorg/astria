use astria_core::{
    protocol::transaction::v1::action::{
        BridgeLock,
        Transfer,
    },
    sequencerblock::v1::block::Deposit,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    accounts::action::{
        check_transfer,
        execute_transfer,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    assets::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeLock {
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
            .get_bridge_account_rollup_id(&self.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;

        let allowed_asset = state
            .get_bridge_account_ibc_asset(&self.to)
            .await
            .wrap_err("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        let source_transaction_id = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .transaction_id;
        let source_action_index = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .source_action_index;

        // map asset to trace prefixed asset for deposit, if it is not already
        let deposit_asset = match self.asset.as_trace_prefixed() {
            Some(asset) => asset.clone(),
            None => state
                .map_ibc_to_trace_prefixed_asset(&allowed_asset)
                .await
                .wrap_err("failed to map IBC asset to trace prefixed asset")?
                .ok_or_eyre("mapping from IBC prefixed bridge asset to trace prefixed not found")?,
        };

        let deposit = Deposit {
            bridge_address: self.to,
            rollup_id,
            amount: self.amount,
            asset: deposit_asset.into(),
            destination_chain_address: self.destination_chain_address.clone(),
            source_transaction_id,
            source_action_index,
        };
        let deposit_abci_event = create_deposit_event(&deposit);

        let transfer_action = Transfer {
            to: self.to,
            asset: self.asset.clone(),
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
mod tests {
    use astria_core::{
        primitive::v1::{
            asset,
            TransactionId,
        },
        protocol::transaction::v1::action::BridgeLock,
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::{
            AddressBytes,
            StateWriteExt as _,
        },
        address::StateWriteExt as _,
        app::ActionHandler as _,
        assets::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            nria,
            ASTRIA_PREFIX,
        },
        bridge::{
            StateReadExt,
            StateWriteExt as _,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn bridge_lock_maps_ibc_to_trace_prefixed_for_deposit() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let trace_asset = "trace_asset"
            .parse::<asset::denom::TracePrefixed>()
            .unwrap();
        let ibc_asset = trace_asset.to_ibc_prefixed();
        let transfer_amount = 100;
        let bridge_address = astria_address(&[3; 20]);
        let from_address = astria_address(&[1; 20]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state
            .put_bridge_account_rollup_id(&bridge_address, [0; 32].into())
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, ibc_asset)
            .unwrap();
        state.put_ibc_asset(trace_asset.clone()).unwrap();
        state
            .put_account_balance(&from_address, &trace_asset, transfer_amount)
            .unwrap();

        let bridge_lock_action = BridgeLock {
            to: bridge_address,
            amount: transfer_amount,
            asset: ibc_asset.into(),
            fee_asset: nria().into(),
            destination_chain_address: "ethan_was_here".to_string(),
        };

        bridge_lock_action
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let deposits = state
            .get_cached_block_deposits()
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(deposits.len(), 1);
        assert!(deposits[0].asset.as_trace_prefixed().is_some());
    }
}
