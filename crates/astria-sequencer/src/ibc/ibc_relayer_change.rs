use astria_core::protocol::transaction::v1::action::IbcRelayerChange;
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    address::StateReadExt as _,
    app::ActionHandler,
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

#[async_trait]
impl ActionHandler for IbcRelayerChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(
        &self,
        mut state: S,
        context: crate::transaction::Context,
    ) -> Result<()> {
        let from = context.address_bytes;
        match self {
            IbcRelayerChange::Addition(addr) | IbcRelayerChange::Removal(addr) => {
                state.ensure_base_prefix(addr).await.wrap_err(
                    "failed check for base prefix of provided address to be added/removed",
                )?;
            }
        }

        let ibc_sudo_address = state
            .get_ibc_sudo_address()
            .await
            .wrap_err("failed to get IBC sudo address")?;
        ensure!(
            ibc_sudo_address == from,
            "unauthorized address for IBC relayer change"
        );

        match self {
            IbcRelayerChange::Addition(address) => {
                state
                    .put_ibc_relayer_address(address)
                    .wrap_err("failed to put IBC relayer address")?;
            }
            IbcRelayerChange::Removal(address) => {
                state.delete_ibc_relayer_address(address);
            }
        }
        Ok(())
    }
}
