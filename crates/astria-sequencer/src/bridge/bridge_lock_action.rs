use anyhow::{
    Context as _,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    transaction::action::{
        BridgeLockAction,
        TransferAction,
    },
    Address,
};
use tracing::instrument;

use crate::{
    accounts::action::transfer_check_stateful,
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
            to: self.to.clone(),
            asset_id: self.asset_id.clone(),
            amount: self.amount,
            fee_asset_id: self.fee_asset_id.clone(),
        };

        // this performs the same checks as a normal `TransferAction`,
        // but without the check that prevents transferring to a bridge account,
        // as we are explicitly transferring to a bridge account here.
        transfer_check_stateful(&transfer_action, state, from).await
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let transfer_action = TransferAction {
            to: self.to.clone(),
            asset_id: self.asset_id.clone(),
            amount: self.amount,
            fee_asset_id: self.fee_asset_id.clone(),
        };

        transfer_action
            .execute(state, from)
            .await
            .context("failed to execute bridge lock action as transfer action")?;
        Ok(())
    }
}
