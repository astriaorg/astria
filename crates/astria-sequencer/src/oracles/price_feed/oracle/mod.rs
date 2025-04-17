pub(crate) mod currency_pair_strategy;
pub(crate) mod state_ext;
pub(crate) mod storage;

use astria_core::oracles::price_feed::oracle::v2::{
    CurrencyPairState,
    GenesisState,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use state_ext::StateWriteExt;

pub(crate) fn handle_genesis<S: StateWriteExt>(
    mut state: S,
    oracle_genesis: &GenesisState,
) -> Result<()> {
    for currency_pair in &oracle_genesis.currency_pair_genesis {
        let currency_pair_state = CurrencyPairState {
            id: currency_pair.id(),
            nonce: currency_pair.nonce(),
            price: currency_pair.currency_pair_price().clone(),
        };
        state
            .put_currency_pair_state(currency_pair.currency_pair().clone(), currency_pair_state)
            .wrap_err("failed to write currency pair to state")?;
    }

    state
        .put_next_currency_pair_id(oracle_genesis.next_id)
        .wrap_err("failed to put next currency pair id")?;
    state
        .put_num_currency_pairs(oracle_genesis.currency_pair_genesis.len() as u64)
        .wrap_err("failed to put number of currency pairs")
}
