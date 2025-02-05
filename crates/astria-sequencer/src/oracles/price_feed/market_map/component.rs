use std::sync::Arc;

use astria_core::{
    oracles::price_feed::market_map::v2::GenesisState,
    protocol::genesis::v1::GenesisAppState,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
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

impl MarketMapComponent {
    pub(crate) fn handle_genesis<S: StateWriteExt>(
        mut state: S,
        market_map_genesis: &GenesisState,
    ) -> Result<()> {
        state
            .put_market_map(market_map_genesis.market_map.clone())
            .wrap_err("failed to put market map")?;
        state
            .put_params(market_map_genesis.params.clone())
            .wrap_err("failed to put params")
    }
}

#[async_trait::async_trait]
impl Component for MarketMapComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "MarketMapComponent::init_chain", skip_all, err)]
    async fn init_chain<S: StateWriteExt>(state: S, app_state: &Self::AppState) -> Result<()> {
        if let Some(price_feed) = app_state.price_feed() {
            Self::handle_genesis(state, price_feed.market_map())?;
        }

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
