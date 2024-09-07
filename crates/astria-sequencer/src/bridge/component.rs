use std::sync::Arc;

use anyhow::Result;
use astria_core::protocol::genesis::v1alpha1::GenesisAppState;
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use super::state_ext::StateWriteExt;
use crate::component::Component;

#[derive(Default)]
pub(crate) struct BridgeComponent;

#[async_trait::async_trait]
impl Component for BridgeComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "BridgeComponent::init_chain", skip_all)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // No need to add context as these `put` methods already report sufficient context on error.
        state.put_init_bridge_account_base_fee(app_state.fees().init_bridge_account_base_fee)?;
        state.put_bridge_lock_byte_cost_multiplier(
            app_state.fees().bridge_lock_byte_cost_multiplier,
        )?;
        state.put_bridge_sudo_change_base_fee(app_state.fees().bridge_sudo_change_fee)
    }

    #[instrument(name = "BridgeComponent::begin_block", skip_all)]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "BridgeComponent::end_block", skip_all)]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
