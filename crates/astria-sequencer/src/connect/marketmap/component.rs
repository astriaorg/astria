use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use super::state_ext::StateWriteExt as _;
use crate::component::Component;

#[derive(Default)]
pub(crate) struct MarketMapComponent;

#[async_trait::async_trait]
impl Component for MarketMapComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "MarketMapComponent::init_chain", skip_all, err)]
    async fn init_chain<S: StateWrite>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        if let Some(connect) = app_state.connect() {
            // TODO: put market map authorities and admin in state;
            // only required for related actions however

            state
                .put_market_map(connect.market_map().market_map.clone())
                .wrap_err("failed to put market map")?;
            state
                .put_params(connect.market_map().params.clone())
                .wrap_err("failed to put params")?;
        }

        Ok(())
    }

    #[instrument(name = "MarketMapComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "MarketMapComponent::end_block", skip(_state))]
    async fn end_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
