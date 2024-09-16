use astria_core::protocol::transaction::v1alpha1::action::IbcRelayerChangeAction;
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
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for IbcRelayerChangeAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        match self {
            IbcRelayerChangeAction::Addition(addr) | IbcRelayerChangeAction::Removal(addr) => {
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
            IbcRelayerChangeAction::Addition(address) => {
                // No need to add context as this method already reports sufficient context on
                // error.
                state.put_ibc_relayer_address(address)?;
            }
            IbcRelayerChangeAction::Removal(address) => {
                state.delete_ibc_relayer_address(address);
            }
        }
        Ok(())
    }
}
