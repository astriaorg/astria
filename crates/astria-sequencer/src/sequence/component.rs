use std::sync::Arc;

use anyhow::Result;
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

/// Default base fee for a [`SequenceAction`].
const DEFAULT_SEQUENCE_ACTION_BASE_FEE: u128 = 32;

/// Default multiplier for the cost of a byte in a [`SequenceAction`].
const DEFAULT_SEQUENCE_ACTION_BYTE_COST_MULTIPLIER: u128 = 1;

#[derive(Default)]
pub(crate) struct SequenceComponent;

#[async_trait::async_trait]
impl Component for SequenceComponent {
    type AppState = GenesisState;

    #[instrument(name = "SequenceComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        state.put_sequence_action_base_fee(DEFAULT_SEQUENCE_ACTION_BASE_FEE);
        state
            .put_sequence_action_byte_cost_multiplier(DEFAULT_SEQUENCE_ACTION_BYTE_COST_MULTIPLIER);
        Ok(())
    }

    #[instrument(name = "SequenceComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "SequenceComponent::end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
