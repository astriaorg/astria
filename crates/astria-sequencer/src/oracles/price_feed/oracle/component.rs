use std::sync::Arc;

use astria_core::{
    oracles::price_feed::oracle::v2::{
        CurrencyPairState,
        GenesisState,
    },
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
pub(crate) struct OracleComponent;

impl OracleComponent {
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
}

#[async_trait::async_trait]
impl Component for OracleComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "OracleComponent::init_chain", skip_all, err)]
    async fn init_chain<S: StateWriteExt>(state: S, app_state: &Self::AppState) -> Result<()> {
        if let Some(price_feed) = app_state.price_feed() {
            Self::handle_genesis(state, price_feed.oracle())?;
        }
        Ok(())
    }

    #[instrument(name = "OracleComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "OracleComponent::end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
