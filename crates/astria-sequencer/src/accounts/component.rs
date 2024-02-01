use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use super::state_ext::StateWriteExt;
use crate::{
    asset::get_native_asset,
    component::Component,
    genesis::GenesisState,
};

#[derive(Default)]
pub(crate) struct AccountsComponent;

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisState;

    #[instrument(name = "AccountsComponent:init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        let native_asset = get_native_asset();
        for account in &app_state.accounts {
            state
                .put_account_balance(account.address, native_asset.id(), account.balance)
                .context("failed writing account balance to state")?;
        }
        state
            .put_ibc_sudo_address(app_state.ibc_sudo_address)
            .context("failed to set IBC sudo key")?;
        Ok(())
    }

    #[instrument(name = "AccountsComponent:begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "AccountsComponent:end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
