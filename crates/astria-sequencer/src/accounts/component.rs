use std::sync::Arc;

use anyhow::Result;
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use super::state_ext::StateWriteExt;
use crate::{
    app::GenesisState,
    component::Component,
};

#[derive(Default)]
pub struct AccountsComponent;

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisState;

    #[instrument(name = "AccountsComponent:init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        for (address, balance) in &app_state.accounts {
            state.put_account_balance(address, *balance)?;
        }
        Ok(())
    }

    #[instrument(name = "AccountsComponent:begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) {
    }

    #[instrument(name = "AccountsComponent:end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(_state: &mut Arc<S>, _end_block: &EndBlock) {}
}
