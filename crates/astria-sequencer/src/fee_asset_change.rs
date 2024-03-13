use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::sequencer::v1::{
    transaction::action::FeeAssetChangeAction,
    Address,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};

use crate::{
    authority::state_ext::StateReadExt as _,
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait]
impl ActionHandler for FeeAssetChangeAction {
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S, from: Address) -> Result<()> {
        let authority_sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get authority sudo address")?;
        ensure!(
            authority_sudo_address == from,
            "unauthorized address for fee asset change"
        );
        Ok(())
    }

    async fn execute<S: StateWrite>(&self, state: &mut S, _from: Address) -> Result<()> {
        match self {
            FeeAssetChangeAction::Addition(asset) => {
                state.put_allowed_fee_asset(*asset);
            }
            FeeAssetChangeAction::Removal(asset) => {
                state.delete_allowed_fee_asset(*asset);

                if state.get_allowed_fee_assets().await?.is_empty() {
                    bail!("cannot remove last allowed fee asset");
                }
            }
        }
        Ok(())
    }
}
