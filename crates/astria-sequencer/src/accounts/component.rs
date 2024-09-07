use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use astria_core::protocol::genesis::v1alpha1::GenesisAppState;
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use crate::{
    accounts,
    assets,
    component::Component,
};

#[derive(Default)]
pub(crate) struct AccountsComponent;

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "AccountsComponent::init_chain", skip_all)]
    async fn init_chain<S>(mut state: S, app_state: &Self::AppState) -> Result<()>
    where
        S: accounts::StateWriteExt + assets::StateReadExt,
    {
        let native_asset = state
            .get_native_asset()
            .await
            .context("failed to read native asset from state")?;
        for account in app_state.accounts() {
            state
                .put_account_balance(&account.address, &native_asset, account.balance)
                .context("failed writing account balance to state")?;
        }

        state
            .put_transfer_base_fee(app_state.fees().transfer_base_fee)
            .context("failed to put transfer base fee")?;
        Ok(())
    }

    #[instrument(name = "AccountsComponent::begin_block", skip_all)]
    async fn begin_block<S: accounts::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "AccountsComponent::end_block", skip_all)]
    async fn end_block<S: accounts::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
