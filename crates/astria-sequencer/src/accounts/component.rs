use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::Result;
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
    async fn init_chain<S>(_state: S, _app_state: &Self::AppState) -> Result<()>
    where
        S: accounts::StateWriteExt + assets::StateReadExt,
    {
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
