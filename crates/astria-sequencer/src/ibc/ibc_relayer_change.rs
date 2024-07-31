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

use crate::{
    address,
    ibc,
    transaction::action_handler::ActionHandler,
};

#[async_trait]
impl ActionHandler for IbcRelayerChangeAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: ibc::StateReadExt + address::StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        match self {
            IbcRelayerChangeAction::Addition(addr) | IbcRelayerChangeAction::Removal(addr) => {
                state.ensure_base_prefix(addr).await.context(
                    "failed check for base prefix of provided address to be added/removed",
                )?;
            }
        }

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

    async fn execute<S: ibc::StateWriteExt>(&self, state: &mut S, _from: Address) -> Result<()> {
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
