use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::InitBridgeAccount,
};
use astria_eyre::eyre::{
    bail,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    address::StateReadExt as _,
    app::ActionHandler,
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for InitBridgeAccount {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        if let Some(withdrawer_address) = &self.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of withdrawer address")?;
        }
        if let Some(sudo_address) = &self.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .wrap_err("failed check for base prefix of sudo address")?;
        }

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
        if state
            .get_bridge_account_rollup_id(&from)
            .await
            .wrap_err("failed getting rollup ID of bridge account")?
            .is_some()
        {
            bail!("bridge account already exists");
        }

        state
            .put_bridge_account_rollup_id(&from, self.rollup_id)
            .wrap_err("failed to put bridge account rollup id")?;
        state
            .put_bridge_account_ibc_asset(&from, &self.asset)
            .wrap_err("failed to put asset ID")?;
        state.put_bridge_account_sudo_address(
            &from,
            self.sudo_address.map_or(from, Address::bytes),
        )?;
        state.put_bridge_account_withdrawer_address(
            &from,
            self.withdrawer_address.map_or(from, Address::bytes),
        )?;

        Ok(())
    }
}
