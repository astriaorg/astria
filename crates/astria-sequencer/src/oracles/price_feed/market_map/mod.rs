pub(crate) mod state_ext;
pub(crate) mod storage;

use astria_core::oracles::price_feed::market_map::v2::GenesisState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use state_ext::StateWriteExt;

use crate::address::StateReadExt as _;

pub(crate) async fn handle_genesis<S: StateWriteExt>(
    mut state: S,
    market_map_genesis: &GenesisState,
) -> Result<()> {
    for address in &market_map_genesis.params.market_authorities {
        state
            .ensure_base_prefix(address)
            .await
            .wrap_err("failed check for base prefix of market authority address")?;
    }
    state
        .ensure_base_prefix(&market_map_genesis.params.admin)
        .await
        .wrap_err("failed check for base prefix of market map admin address")?;
    state
        .put_market_map(market_map_genesis.market_map.clone())
        .wrap_err("failed to put market map")?;
    state
        .put_params(market_map_genesis.params.clone())
        .wrap_err("failed to put params")
}
