use astria_core::{
    protocol::transaction::v1alpha1::action::BridgeSudoChangeAction,
    Protobuf as _,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    app::ActionHandler,
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};
#[async_trait::async_trait]
impl ActionHandler for BridgeSudoChangeAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

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

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

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

        let fee = state
            .get_bridge_sudo_change_base_fee()
            .await
            .wrap_err("failed to get bridge sudo change fee")?;
        state
            .get_and_increase_block_fees(&self.fee_asset, fee, Self::full_name())
            .await
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(&self.bridge_address, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for bridge sudo change fee")?;

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
    use astria_core::primitive::v1::{
        asset,
        TransactionId,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        test_utils::{
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
    async fn fails_with_unauthorized_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        state.put_allowed_fee_asset(&asset).unwrap();

        let bridge_address = astria_address(&[99; 20]);
        let sudo_address = astria_address(&[98; 20]);
        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let action = BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: asset.clone(),
        };

        assert!(
            action
                .check_and_execute(state)
                .await
                .unwrap_err()
                .to_string()
                .contains("unauthorized for bridge sudo change action")
        );
    }

    #[tokio::test]
    async fn executes() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[98; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_bridge_sudo_change_base_fee(10).unwrap();

        let fee_asset = test_asset();
        state.put_allowed_fee_asset(&fee_asset).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let new_sudo_address = astria_address(&[98; 20]);
        let new_withdrawer_address = astria_address(&[97; 20]);
        state
            .put_account_balance(&bridge_address, &fee_asset, 10)
            .unwrap();

        let action = BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: Some(new_sudo_address),
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset,
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
}
