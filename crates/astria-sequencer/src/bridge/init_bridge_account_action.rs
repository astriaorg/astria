use anyhow::{
    anyhow,
    ensure,
    Context as _,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    transaction::action::InitBridgeAccountAction,
    Address,
};
use tracing::instrument;

use crate::{
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

// TODO: add a fee for this
#[async_trait::async_trait]
impl ActionHandler for InitBridgeAccountAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
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
        if state.get_bridge_account_rollup_id(from).await?.is_some() {
            return Err(anyhow!("bridge account already exists"));
        }

        ensure!(
            !self.asset_ids.is_empty(),
            "must initialize with at least one asset ID",
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        state.put_bridge_account_rollup_id(from, self.rollup_id);
        state
            .put_bridge_account_asset_ids(from, &self.asset_ids)
            .context("failed to put asset IDs")?;
        Ok(())
    }
}
