use std::sync::Arc;

use astria_core::protocol::genesis::v1alpha1::GenesisAppState;
use astria_eyre::eyre::Result;
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
    async fn init_chain<S: StateWriteExt>(
        mut _state: S,
        _app_state: &Self::AppState,
    ) -> Result<()> {
        Ok(())
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
