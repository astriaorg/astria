use astria_core::{
    primitive::v1::asset::Denom,
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
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    address::StateReadExt as _,
    assets::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

#[async_trait]
impl ActionHandler for BridgeLock {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;

        // check that the asset to be transferred matches the bridge account asset.
        // this also implicitly ensures the recipient is a bridge account.
        let allowed_asset = state
            .get_bridge_account_ibc_asset(&self.to)
            .await
            .wrap_err("failed to get bridge account asset ID; account is not a bridge account")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        execute_bridge_lock(self, state).await?;
        Ok(())
    }
}

pub(super) async fn execute_bridge_lock<S: StateWrite>(
    bridge_lock: &BridgeLock,
    mut state: S,
) -> Result<()> {
    let from = state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action")
        .address_bytes();
    let rollup_id = state
        .get_bridge_account_rollup_id(&bridge_lock.to)
        .await
        .wrap_err("failed to get bridge account rollup id")?
        .ok_or_eyre("bridge lock must be sent to a bridge account")?;

    let source_transaction_id = state
        .get_transaction_context()
        .expect("current source should be set before executing action")
        .transaction_id;
    let source_action_index = state
        .get_transaction_context()
        .expect("current source should be set before executing action")
        .position_in_transaction;

    // map asset to trace prefixed asset for deposit, if it is not already
    let deposit_asset = match &bridge_lock.asset {
        Denom::TracePrefixed(asset) => asset.clone(),
        Denom::IbcPrefixed(asset) => state
            .map_ibc_to_trace_prefixed_asset(asset)
            .await
            .wrap_err("failed to map IBC asset to trace prefixed asset")?
            .ok_or_eyre("mapping from IBC prefixed bridge asset to trace prefixed not found")?,
    };

    let deposit = Deposit {
        bridge_address: bridge_lock.to,
        rollup_id,
        amount: bridge_lock.amount,
        asset: deposit_asset.into(),
        destination_chain_address: bridge_lock.destination_chain_address.clone(),
        source_transaction_id,
        source_action_index,
    };
    let deposit_abci_event = create_deposit_event(&deposit);

    let transfer_action = Transfer {
        to: bridge_lock.to,
        asset: bridge_lock.asset.clone(),
        amount: bridge_lock.amount,
        fee_asset: bridge_lock.fee_asset.clone(),
    };

    check_transfer(&transfer_action, &from, &state).await?;
    execute_transfer(&transfer_action, &from, &mut state).await?;

    state.cache_deposit_event(deposit);
    state.record(deposit_abci_event);
    Ok(())
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
        action_handler::ActionHandler as _,
        address::StateWriteExt as _,
        assets::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
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
            position_in_transaction: 0,
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

    #[tokio::test]
    async fn bridge_lock_fails_if_not_sent_to_bridge_account() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let bridge_lock_action = BridgeLock {
            to: astria_address(&[3; 20]),
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: "ethan_was_here".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "failed to get bridge account asset ID; account is not a bridge account",
        );
    }

    #[tokio::test]
    async fn bridge_lock_fails_if_destination_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();

        let bridge_lock_action = BridgeLock {
            to: astria_address(&[3; 20]),
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: "ethan_was_here".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            &format!(
                "address has prefix `{ASTRIA_PREFIX}` but only `{different_prefix}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn bridge_lock_fails_if_asset_is_not_allowed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_asset = "trace_asset"
            .parse::<asset::denom::TracePrefixed>()
            .unwrap();
        let action_asset = nria();
        let bridge_address = astria_address(&[3; 20]);
        let from_address = astria_address(&[1; 20]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state
            .put_bridge_account_rollup_id(&bridge_address, [0; 32].into())
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, bridge_asset)
            .unwrap();

        let bridge_lock_action = BridgeLock {
            to: astria_address(&[3; 20]),
            amount: 1,
            asset: action_asset.into(),
            fee_asset: nria().into(),
            destination_chain_address: "ethan_was_here".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "asset ID is not authorized for transfer to bridge account",
        );
    }

    #[tokio::test]
    async fn bridge_lock_fails_if_ibc_asset_cannot_be_mapped_to_trace() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let ibc_asset = nria().to_ibc_prefixed();
        let bridge_address = astria_address(&[3; 20]);
        let from_address = astria_address(&[1; 20]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state
            .put_bridge_account_rollup_id(&bridge_address, [0; 32].into())
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, ibc_asset)
            .unwrap();

        let bridge_lock_action = BridgeLock {
            to: astria_address(&[3; 20]),
            amount: 1,
            asset: ibc_asset.into(),
            fee_asset: nria().into(),
            destination_chain_address: "ethan_was_here".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "mapping from IBC prefixed bridge asset to trace prefixed not found",
        );
    }
}
