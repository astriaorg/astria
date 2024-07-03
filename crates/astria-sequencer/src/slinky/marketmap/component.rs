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
use crate::component::Component;

#[derive(Default)]
pub(crate) struct MarketMapComponent;

#[async_trait::async_trait]
impl Component for MarketMapComponent {
    type AppState = astria_core::sequencer::GenesisState;

    #[instrument(name = "MarketMapComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        state
            .put_market_map(app_state.market_map().market_map.clone())
            .context("failed to put market map")?;
        state
            .put_params(app_state.market_map().params.clone())
            .context("failed to put params")?;
        Ok(())
    }

    #[instrument(name = "MarketMapComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "MarketMapComponent::end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
