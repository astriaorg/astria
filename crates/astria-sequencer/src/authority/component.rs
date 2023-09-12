use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use tendermint::{
    abci::request::{
        BeginBlock,
        EndBlock,
    },
    validator,
};
use tracing::instrument;

use super::state_ext::StateWriteExt;
use crate::{
    component::Component,
    genesis::GenesisState,
};

#[derive(Default)]
pub(crate) struct AuthorityComponent;

#[async_trait::async_trait]
impl Component for AuthorityComponent {
    type AppState = (GenesisState, Vec<validator::Update>);

    #[instrument(name = "AuthorityComponent:init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // set sudo key and initial validator set
        state
            .put_sudo_address(app_state.0.authority_sudo_key)
            .context("failed to set sudo key")?;
        state
            .put_validator_set(app_state.1.clone().try_into()?)
            .context("failed to set validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent:begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) {
    }

    #[instrument(name = "AuthorityComponent:end_block", skip(_state))]
    async fn end_block<S: StateWriteExt + 'static>(_state: &mut Arc<S>, _end_block: &EndBlock) {}
}
