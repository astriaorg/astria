use astria_core::protocol::transaction::v1::action::BridgeSudoChange;
use astria_eyre::eyre::{
    bail,
    ensure,
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
impl ActionHandler for BridgeSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;
        if let Some(new_sudo_address) = &self.new_sudo_address {
            state
                .ensure_base_prefix(new_sudo_address)
                .await
                .wrap_err("failed check for base prefix of new sudo address")?;
        }
        if let Some(new_withdrawer_address) = &self.new_withdrawer_address {
            state
                .ensure_base_prefix(new_withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of new withdrawer address")?;
        }

        // check that the sender of this tx is the authorized sudo address for the bridge account
        let Some(sudo_address) = state
            .get_bridge_account_sudo_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account sudo address")?
        else {
            // TODO: if the sudo address is unset, should we still allow this action
            // if the sender if the bridge address itself?
            bail!("bridge account does not have an associated sudo address");
        };

        ensure!(
            sudo_address == from,
            "unauthorized for bridge sudo change action",
        );

        if let Some(sudo_address) = self.new_sudo_address {
            state
                .put_bridge_account_sudo_address(&self.bridge_address, sudo_address)
                .wrap_err("failed to put bridge account sudo address")?;
        }

        if let Some(withdrawer_address) = self.new_withdrawer_address {
            state
                .put_bridge_account_withdrawer_address(&self.bridge_address, withdrawer_address)
                .wrap_err("failed to put bridge account withdrawer address")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            Address,
            TransactionId,
        },
        protocol::transaction::v1::action::BridgeSudoChange,
    };
    use cnidarium::StateDelta;

    use crate::{
        action_handler::{
            impls::test_utils::test_asset,
            ActionHandler as _,
        },
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        bridge::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        fees::StateWriteExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn bridge_sudo_change_fails_with_unauthorized_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        state.put_allowed_fee_asset(&asset).unwrap();

        let bridge_address = astria_address(&[99; 20]);
        let sudo_address = astria_address(&[98; 20]);
        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: asset.clone(),
        };

        assert_eyre_error(
            &action.check_and_execute(state).await.unwrap_err(),
            "unauthorized for bridge sudo change action",
        );
    }

    #[tokio::test]
    async fn bridge_sudo_change_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[98; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let new_sudo_address = astria_address(&[98; 20]);
        let new_withdrawer_address = astria_address(&[97; 20]);

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(new_sudo_address),
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset: test_asset(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(
            state
                .get_bridge_account_sudo_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_sudo_address.bytes()),
        );
        assert_eq!(
            state
                .get_bridge_account_withdrawer_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_withdrawer_address.bytes()),
        );
    }

    #[tokio::test]
    async fn bridge_sudo_change_fails_if_bridge_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[98; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(astria_address(&[98; 20])),
            new_withdrawer_address: Some(astria_address(&[97; 20])),
            fee_asset: test_asset(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            &format!(
                "address has prefix `{ASTRIA_PREFIX}` but only `{different_prefix}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn bridge_sudo_change_fails_if_new_sudo_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[98; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let different_prefix = "different_prefix";
        let new_sudo_address = Address::builder()
            .array([98; 20])
            .prefix(different_prefix)
            .try_build()
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(new_sudo_address),
            new_withdrawer_address: Some(astria_address(&[97; 20])),
            fee_asset: test_asset(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn bridge_sudo_change_fails_if_new_withdrawer_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[98; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let different_prefix = "different_prefix";
        let new_withdrawer_address = Address::builder()
            .array([97; 20])
            .prefix(different_prefix)
            .try_build()
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(astria_address(&[98; 20])),
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset: test_asset(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted"
            ),
        );
    }
}
