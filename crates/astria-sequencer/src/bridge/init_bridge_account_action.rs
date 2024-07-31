use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::InitBridgeAccountAction,
};
use tracing::instrument;

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    address,
    assets::StateReadExt as _,
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
impl ActionHandler for InitBridgeAccountAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + address::StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        if let Some(withdrawer_address) = &self.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .context("failed check for base prefix of withdrawer address")?;
        }
        if let Some(sudo_address) = &self.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .context("failed check for base prefix of sudo address")?;
        }

        ensure!(
            state.is_allowed_fee_asset(&self.fee_asset).await?,
            "invalid fee asset",
        );

        let fee = state
            .get_init_bridge_account_base_fee()
            .await
            .context("failed to get base fee for initializing bridge account")?;

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
            bail!("bridge account already exists");
        }

        let balance = state
            .get_account_balance(from, &self.fee_asset)
            .await
            .context("failed getting `from` account balance for fee payment")?;

        ensure!(
            balance >= fee,
            "insufficient funds for bridge account initialization",
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let fee = state
            .get_init_bridge_account_base_fee()
            .await
            .context("failed to get base fee for initializing bridge account")?;

        state.put_bridge_account_rollup_id(&from, &self.rollup_id);
        state
            .put_bridge_account_ibc_asset(&from, &self.asset)
            .context("failed to put asset ID")?;
        state.put_bridge_account_sudo_address(&from, &self.sudo_address.unwrap_or(from));
        state
            .put_bridge_account_withdrawer_address(&from, &self.withdrawer_address.unwrap_or(from));

        state
            .decrease_balance(from, &self.fee_asset, fee)
            .await
            .context("failed to deduct fee from account balance")?;
        Ok(())
    }
}
