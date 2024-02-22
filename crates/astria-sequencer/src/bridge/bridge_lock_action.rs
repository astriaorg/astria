use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    block::Deposit,
    transaction::action::{
        BridgeLockAction,
        TransferAction,
    },
    Address,
};
use tracing::instrument;

use crate::{
    accounts::action::transfer_check_stateful,
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
        ensure!(
            state.get_bridge_account_rollup_id(self.to).await?.is_some(),
            "bridge lock must be sent to a bridge account",
        );

        let allowed_asset_ids = state
            .get_bridge_account_asset_ids(from)
            .await
            .context("failed to get bridge account asset IDs")?;
        ensure!(
            allowed_asset_ids.contains(&self.asset_id),
            "asset ID is not authorized for transfer to bridge account",
        );

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
            .get_bridge_account_rollup_id(self.to)
            .await?
            .expect("recipient must be a bridge account; this is a bug in check_stateful");

        let deposit = Deposit {
            bridge_address: self.to,
            rollup_id,
            asset_id: self.asset_id,
            amount: self.amount,
            destination_chain_address: self.destination_chain_address.clone(),
        };
        state
            .put_deposit_event(deposit)
            .await
            .context("failed to put deposit event into state")?;
        Ok(())
    }
}
