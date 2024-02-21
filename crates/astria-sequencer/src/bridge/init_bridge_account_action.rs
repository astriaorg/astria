use anyhow::{
    anyhow,
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    transaction::action::InitBridgeAccountAction,
    Address,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for InitBridgeAccountAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        if let Some(_) = state.get_bridge_account_rollup_id(from).await? {
            return Err(anyhow!("bridge account already exists"));
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        state.put_bridge_account_rollup_id(from, self.rollup_id.clone());
        state
            .put_bridge_account_balance(from, 0)
            .context("failed to put initial bridge account balance")?;
        Ok(())
    }
}
