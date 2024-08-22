use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use astria_core::{
    protocol::genesis::v1alpha1::GenesisAppState,
    slinky::oracle::v1::CurrencyPairState,
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

#[async_trait::async_trait]
impl Component for OracleComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "OracleComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        for currency_pair in &app_state.oracle().currency_pair_genesis {
            let currency_pair_state = CurrencyPairState {
                id: currency_pair.id(),
                nonce: currency_pair.nonce(),
                price: currency_pair.currency_pair_price().clone(),
            };
            state
                .put_currency_pair_state(currency_pair.currency_pair(), currency_pair_state)
                .context("failed to put currency pair")?;
            tracing::info!(
                "put currency pair: {}",
                currency_pair.currency_pair().to_string()
            );
        }

        state
            .put_next_currency_pair_id(app_state.oracle().next_id)
            .context("failed to put next currency pair id")?;
        state
            .put_num_currency_pairs(app_state.oracle().currency_pair_genesis.len() as u64)
            .context("failed to put number of currency pairs")?;
        state
            .put_num_removed_currency_pairs(0)
            .context("failed to put number of removed currency pairs")?;
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
