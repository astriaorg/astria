use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::FeeAssetChangeAction,
};
use async_trait::async_trait;

use crate::{
    assets,
    assets::StateReadExt as _,
    authority,
    transaction::action_handler::ActionHandler,
};

#[async_trait]
impl ActionHandler for FeeAssetChangeAction {
    async fn check_stateful<S: authority::StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
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

    async fn execute<S: assets::StateWriteExt>(&self, state: &mut S, _from: Address) -> Result<()> {
        match self {
            FeeAssetChangeAction::Addition(asset) => {
                state.put_allowed_fee_asset(asset);
            }
            FeeAssetChangeAction::Removal(asset) => {
                state.delete_allowed_fee_asset(asset);

                if state
                    .get_allowed_fee_assets()
                    .await
                    .context("failed to retrieve allowed fee assets")?
                    .is_empty()
                {
                    bail!("cannot remove last allowed fee asset");
                }
            }
        }
        Ok(())
    }
}
