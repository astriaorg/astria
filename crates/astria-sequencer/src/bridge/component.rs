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

/// Default base fee for a [`InitBridgeAccountAction`].
const DEFAULT_INIT_BRIDGE_ACCOUNT_BASE_FEE: u128 = 48;

/// Default multiplier for the cost of a byte in a [`BridgeLockAction`].
///
/// Note that the base fee for a [`BridgeLockAction`] is the same as the
/// base fee for a [`TransferAction`].
const DEFAULT_BRIDGE_LOCK_BYTE_COST_MULTIPLIER: u128 = 1;

#[derive(Default)]
pub(crate) struct BridgeComponent;

#[async_trait::async_trait]
impl Component for BridgeComponent {
    type AppState = GenesisState;

    #[instrument(name = "BridgeComponent::init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        state.put_init_bridge_account_base_fee(DEFAULT_INIT_BRIDGE_ACCOUNT_BASE_FEE);
        state.put_bridge_lock_byte_cost_multiplier(DEFAULT_BRIDGE_LOCK_BYTE_COST_MULTIPLIER);
        Ok(())
    }

    #[instrument(name = "BridgeComponent::begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "BridgeComponent::end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
