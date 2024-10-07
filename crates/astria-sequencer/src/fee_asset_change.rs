use astria_core::protocol::transactions::v1alpha1::action::FeeAssetChange;
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    app::ActionHandler,
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    authority::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for FeeAssetChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        let authority_sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get authority sudo address")?;
        ensure!(
            authority_sudo_address == from,
            "unauthorized address for fee asset change"
        );
        match self {
            FeeAssetChange::Addition(asset) => {
                state
                    .put_allowed_fee_asset(asset)
                    .context("failed to write allowed fee asset to state")?;
            }
            FeeAssetChange::Removal(asset) => {
                state.delete_allowed_fee_asset(asset);

                if state
                    .get_allowed_fee_assets()
                    .await
                    .wrap_err("failed to retrieve allowed fee assets")?
                    .is_empty()
                {
                    bail!("cannot remove last allowed fee asset");
                }
            }
        }
        Ok(())
    }
}
