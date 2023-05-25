use std::sync::Arc;

use anyhow::Result;
use penumbra_component::Component;
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};

use super::state_ext::StateWriteExt;
use crate::app::GenesisState;

#[derive(Default)]
pub struct AccountsComponent;

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisState;

    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) {
        for (address, balance) in &app_state.accounts {
            state.put_account_state(address, *balance, 0);
        }
    }

    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) {
    }

    async fn end_block<S: StateWriteExt + 'static>(_state: &mut Arc<S>, _end_block: &EndBlock) {}

    // TODO: are we going to have epochs? might need to write out own Component trait
    async fn end_epoch<S: StateWriteExt + 'static>(_state: &mut Arc<S>) -> Result<()> {
        Ok(())
    }
}
