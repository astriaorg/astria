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
    component::Component,
    genesis::GenesisState,
};

// TODO rename to MarketMapComponent
// or do we want to put all slinky stuff in one component?
#[derive(Default)]
pub(crate) struct SlinkyComponent;

#[async_trait::async_trait]
impl Component for SlinkyComponent {
    type AppState = GenesisState;

    #[instrument(name = "SlinkyComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        if let Some(market_map) = &app_state.market_map.market_map {
            state
                .put_market_map(market_map.clone())
                .context("failed to put market map")?;
        }

        if let Some(params) = &app_state.market_map.params {
            state
                .put_params(params.clone())
                .context("failed to put params")?;
        }

        Ok(())
    }

    #[instrument(name = "SlinkyComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "SlinkyComponent::end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
