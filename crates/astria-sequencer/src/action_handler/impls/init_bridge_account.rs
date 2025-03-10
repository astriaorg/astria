use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::action::InitBridgeAccount,
};
use astria_eyre::eyre::{
    bail,
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
    action_handler::ActionHandler,
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for InitBridgeAccount {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        if let Some(withdrawer_address) = &self.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of withdrawer address")?;
        }
        if let Some(sudo_address) = &self.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .wrap_err("failed check for base prefix of sudo address")?;
        }

        // this prevents the address from being registered as a bridge account
        // if it's been previously initialized as a bridge account.
        //
        // however, there is no prevention of initializing an account as a bridge
        // account that's already been used as a normal EOA.
        //
        // the implication is that the account might already have a balance, nonce, etc.
        // before being converted into a bridge account.
        //
        // after the account becomes a bridge account, it can no longer receive funds
        // via `TransferAction`, only via `BridgeLockAction`.
        if state
            .get_bridge_account_rollup_id(&from)
            .await
            .wrap_err("failed getting rollup ID of bridge account")?
            .is_some()
        {
            bail!("bridge account already exists");
        }

        state
            .put_bridge_account_rollup_id(&from, self.rollup_id)
            .wrap_err("failed to put bridge account rollup id")?;
        state
            .put_bridge_account_ibc_asset(&from, &self.asset)
            .wrap_err("failed to put asset ID")?;
        state.put_bridge_account_sudo_address(
            &from,
            self.sudo_address.map_or(from, Address::bytes),
        )?;
        state.put_bridge_account_withdrawer_address(
            &from,
            self.withdrawer_address.map_or(from, Address::bytes),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset::Denom,
        RollupId,
        TransactionId,
    };

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            nria,
            ASTRIA_PREFIX,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn init_bridge_account_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let bridge_address = astria_address(&[1; 20]);
        let sudo_address = astria_address(&[2; 20]);
        let withdrawer_address = astria_address(&[3; 20]);
        let rollup_id = RollupId::new([1; 32]);
        let asset = Denom::from(nria());

        state.put_transaction_context(TransactionContext {
            address_bytes: *bridge_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = InitBridgeAccount {
            rollup_id,
            asset: asset.clone(),
            fee_asset: asset.clone(),
            sudo_address: Some(sudo_address),
            withdrawer_address: Some(withdrawer_address),
        };

        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(
            state
                .get_bridge_account_rollup_id(&bridge_address)
                .await
                .unwrap(),
            Some(rollup_id)
        );
        assert_eq!(
            state
                .get_bridge_account_ibc_asset(&bridge_address)
                .await
                .unwrap(),
            asset.to_ibc_prefixed()
        );
        assert_eq!(
            state
                .get_bridge_account_sudo_address(&bridge_address)
                .await
                .unwrap(),
            Some(*sudo_address.address_bytes())
        );
        assert_eq!(
            state
                .get_bridge_account_withdrawer_address(&bridge_address)
                .await
                .unwrap(),
            Some(*withdrawer_address.address_bytes())
        );
    }

    #[tokio::test]
    async fn init_bridge_account_fails_if_withdrawer_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let bridge_address = astria_address(&[1; 20]);
        let different_prefix = "different_prefix";
        let withdrawer_address = Address::builder()
            .prefix(different_prefix)
            .array([0; 20])
            .try_build()
            .unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *bridge_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = InitBridgeAccount {
            rollup_id: RollupId::new([1; 32]),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: Some(withdrawer_address),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn init_bridge_account_fails_if_sudo_address_is_not_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let bridge_address = astria_address(&[1; 20]);
        let different_prefix = "different_prefix";
        let sudo_address = Address::builder()
            .prefix(different_prefix)
            .array([0; 20])
            .try_build()
            .unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *bridge_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = InitBridgeAccount {
            rollup_id: RollupId::new([1; 32]),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: Some(sudo_address),
            withdrawer_address: None,
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn init_bridge_account_fails_if_bridge_account_already_exists() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let bridge_address = astria_address(&[1; 20]);
        let rollup_id = RollupId::new([1; 32]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *bridge_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();

        let action = InitBridgeAccount {
            rollup_id,
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "bridge account already exists",
        );
    }
}
