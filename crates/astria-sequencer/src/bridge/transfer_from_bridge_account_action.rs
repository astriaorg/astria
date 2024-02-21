use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    transaction::action::TransferFromBridgeAccountAction,
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
impl ActionHandler for TransferFromBridgeAccountAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        let balance = state.get_bridge_account_balance(from).await?;
        ensure!(
            balance >= self.amount,
            "insufficient balance to transfer from bridge account",
        );
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let balance = state.get_bridge_account_balance(from).await?;
        state
            .put_bridge_account_balance(from, balance - self.amount)
            .context("failed to update bridge account balance")?;
        Ok(())
    }
}
