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
    genesis::{
        GenesisState,
        SEQUENCE_BASE_FEE_FIELD_NAME,
        SEQUENCE_BYTE_COST_MULTIPLIER_FIELD_NAME,
    },
};

#[derive(Default)]
pub(crate) struct SequenceComponent;

#[async_trait::async_trait]
impl Component for SequenceComponent {
    type AppState = GenesisState;

    #[instrument(name = "SequenceComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        state.put_sequence_action_base_fee(
            *app_state.fees.get(SEQUENCE_BASE_FEE_FIELD_NAME).expect(
                "genesis `fees` must contain `sequence_base_fee`, as it was validated during \
                 construction",
            ),
        );
        state.put_sequence_action_byte_cost_multiplier(
            *app_state
                .fees
                .get(SEQUENCE_BYTE_COST_MULTIPLIER_FIELD_NAME)
                .expect(
                    "genesis `fees` must contain `sequence_byte_cost_multiplier`, as it was \
                     validated during construction",
                ),
        );
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
