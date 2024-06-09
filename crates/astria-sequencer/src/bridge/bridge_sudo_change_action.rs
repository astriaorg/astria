use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::BridgeSudoChangeAction,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::StateWriteExt as _,
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
impl ActionHandler for BridgeSudoChangeAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ensure!(
            state
                .is_allowed_fee_asset(self.fee_asset_id)
                .await
                .context("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        // check that the sender of this tx is the authorized sudo address for the bridge account
        let Some(sudo_address) = state
            .get_bridge_account_sudo_address(&self.bridge_address)
            .await
            .context("failed to get bridge account sudo address")?
        else {
            // TODO: if the sudo address is unset, should we still allow this action
            // if the sender if the bridge address itself?
            anyhow::bail!("bridge account does not have an associated sudo address");
        };

        ensure!(
            sudo_address == from,
            "unauthorized for bridge sudo change action",
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        let fee = state
            .get_bridge_sudo_change_base_fee()
            .await
            .context("failed to get bridge sudo change fee")?;
        state
            .decrease_balance(self.bridge_address, self.fee_asset_id, fee)
            .await
            .context("failed to decrease balance for bridge sudo change fee")?;

        if let Some(sudo_address) = self.new_sudo_address {
            state.put_bridge_account_sudo_address(&self.bridge_address, &sudo_address);
        }

        if let Some(withdrawer_address) = self.new_withdrawer_address {
            state.put_bridge_account_withdrawer_address(&self.bridge_address, &withdrawer_address);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset::Id;
    use cnidarium::StateDelta;

    use super::*;

    #[tokio::test]
    async fn bridge_sudo_change_check_stateless_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = Id::from_denom("test");
        state.put_allowed_fee_asset(asset_id);

        let bridge_address = crate::astria_address([99; 20]);
        let sudo_address = crate::astria_address([98; 20]);
        state.put_bridge_account_sudo_address(&bridge_address, &sudo_address);

        let action = BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset_id: asset_id,
        };

        action.check_stateful(&state, sudo_address).await.unwrap();
    }

    #[tokio::test]
    async fn bridge_sudo_change_check_stateless_unauthorized() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let asset_id = Id::from_denom("test");
        state.put_allowed_fee_asset(asset_id);

        let bridge_address = crate::astria_address([99; 20]);
        let sudo_address = crate::astria_address([98; 20]);
        state.put_bridge_account_sudo_address(&bridge_address, &sudo_address);

        let action = BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset_id: asset_id,
        };

        assert!(
            action
                .check_stateful(&state, bridge_address)
                .await
                .unwrap_err()
                .to_string()
                .contains("unauthorized for bridge sudo change action")
        );
    }

    #[tokio::test]
    async fn bridge_sudo_change_execute_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        state.put_bridge_sudo_change_base_fee(10);

        let fee_asset_id = Id::from_denom("test");
        let bridge_address = crate::astria_address([99; 20]);
        let new_sudo_address = crate::astria_address([98; 20]);
        let new_withdrawer_address = crate::astria_address([97; 20]);
        state
            .put_account_balance(bridge_address, fee_asset_id, 10)
            .unwrap();

        let action = BridgeSudoChangeAction {
            bridge_address,
            new_sudo_address: Some(new_sudo_address),
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset_id,
        };

        action.execute(&mut state, bridge_address).await.unwrap();

        assert_eq!(
            state
                .get_bridge_account_sudo_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_sudo_address),
        );
        assert_eq!(
            state
                .get_bridge_account_withdrawer_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_withdrawer_address),
        );
    }
}
