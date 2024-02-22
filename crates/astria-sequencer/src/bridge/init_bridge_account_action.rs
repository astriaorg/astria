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
    accounts::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
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

/// Fee charged for a `InitBridgeAccountAction`.
pub(crate) const INIT_BRIDGE_ACCOUNT_FEE: u128 = 48;

#[async_trait::async_trait]
impl ActionHandler for InitBridgeAccountAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ensure!(
            state.is_allowed_fee_asset(self.fee_asset_id).await?,
            "invalid fee asset",
        );

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
        if state.get_bridge_account_rollup_id(&from).await?.is_some() {
            return Err(anyhow!("bridge account already exists"));
        }

        ensure!(
            !self.asset_ids.is_empty(),
            "must initialize with at least one asset ID",
        );

        let balance = state
            .get_account_balance(from, self.fee_asset_id)
            .await
            .context("failed getting `from` account balance for fee payment")?;

        ensure!(
            balance >= INIT_BRIDGE_ACCOUNT_FEE,
            "insufficient funds for bridge account initialization",
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        state.put_bridge_account_rollup_id(&from, self.rollup_id);
        state
            .put_bridge_account_asset_ids(&from, &self.asset_ids)
            .context("failed to put asset IDs")?;

        state
            .decrease_balance(from, self.fee_asset_id, INIT_BRIDGE_ACCOUNT_FEE)
            .await
            .context("failed to deduct fee from account balance")?;
        Ok(())
    }
}
