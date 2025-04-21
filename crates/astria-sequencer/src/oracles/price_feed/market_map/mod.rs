pub(crate) mod state_ext;
pub(crate) mod storage;

use astria_core::oracles::price_feed::market_map::v2::GenesisState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use state_ext::StateWriteExt;

pub(crate) fn handle_genesis<S: StateWriteExt>(
    mut state: S,
    market_map_genesis: &GenesisState,
) -> Result<()> {
    state
        .put_market_map(market_map_genesis.market_map.clone())
        .wrap_err("failed to put market map")
}
