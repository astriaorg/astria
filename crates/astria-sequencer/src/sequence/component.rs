use std::sync::Arc;

use astria_core::protocol::genesis::v1alpha1::GenesisAppState;
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
pub(crate) struct SequenceComponent;

#[async_trait::async_trait]
impl Component for SequenceComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "SequenceComponent::init_chain", skip_all)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        state
            .put_sequence_action_base_fee(app_state.fees().sequence_base_fee)
            .wrap_err("failed to put sequence action base fee")?;
        state
            .put_sequence_action_byte_cost_multiplier(
                app_state.fees().sequence_byte_cost_multiplier,
            )
            .wrap_err("failed to put sequence action byte cost multiplier")
    }

    #[instrument(name = "SequenceComponent::begin_block", skip_all)]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "SequenceComponent::end_block", skip_all)]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
