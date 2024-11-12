use astria_core::protocol::transaction::v1::action::BridgeSudoChange;
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    action_handler::ActionHandler,
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for BridgeSudoChange {
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
