use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::IbcRelayerChangeAction,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};

use crate::{
    ibc::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait]
impl ActionHandler for IbcRelayerChangeAction {
    async fn check_stateless(&self) -> Result<()> {
        match self {
            IbcRelayerChangeAction::Addition(addr) | IbcRelayerChangeAction::Removal(addr) => {
                crate::address::ensure_base_prefix(addr)
<<<<<<< HEAD
                    .context("provided address to be changed is invalid")?;
=======
                    .context("provided address to be added or removed has an unsupported prefix")?;
>>>>>>> 5a3af081c47cf8b8b2d91d3eadd030f86ecf5ec1
            }
        }

        Ok(())
    }

    async fn check_stateful<S: StateRead + 'static>(&self, state: &S, from: Address) -> Result<()> {
        let ibc_sudo_address = state
            .get_ibc_sudo_address()
            .await
            .context("failed to get IBC sudo address")?;
        ensure!(
            ibc_sudo_address == from,
            "unauthorized address for IBC relayer change"
        );
        Ok(())
    }

    async fn execute<S: StateWrite>(&self, state: &mut S, _from: Address) -> Result<()> {
        match self {
            IbcRelayerChangeAction::Addition(address) => {
                state.put_ibc_relayer_address(address);
            }
            IbcRelayerChangeAction::Removal(address) => {
                state.delete_ibc_relayer_address(address);
            }
        }
        Ok(())
    }
}
