use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::Address;
use tendermint::{
    abci::request::{
        BeginBlock,
        EndBlock,
    },
    validator,
};
use tracing::instrument;

use super::state_ext::{
    StateReadExt,
    StateWriteExt,
    ValidatorSet,
};
use crate::component::Component;

#[derive(Default)]
pub(crate) struct AuthorityComponent;

#[derive(Debug)]
pub(crate) struct AuthorityComponentAppState {
    pub(crate) authority_sudo_key: Address,
    pub(crate) genesis_validators: Vec<validator::Update>,
}

#[async_trait::async_trait]
impl Component for AuthorityComponent {
    type AppState = AuthorityComponentAppState;

    #[instrument(name = "AuthorityComponent:init_chain", skip(state))]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // set sudo key and initial validator set
        state
            .put_sudo_address(app_state.authority_sudo_key)
            .context("failed to set sudo key")?;
        state
            .put_validator_set(ValidatorSet::new_from_updates(
                app_state.genesis_validators.clone(),
            ))
            .context("failed to set validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent:begin_block", skip(_state))]
    async fn begin_block<S: StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) {
    }

    #[instrument(name = "AuthorityComponent:end_block", skip(state))]
    async fn end_block<S: StateWriteExt + StateReadExt + 'static>(
        state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) {
        // update validator set
        let validator_updates = state
            .get_validator_updates()
            .await
            .expect("failed getting validator updates");

        let mut current_set = state
            .get_validator_set()
            .await
            .expect("failed getting validator set");
        current_set.apply_updates(validator_updates);

        let state =
            Arc::get_mut(state).expect("must only have one reference to the state; this is a bug");
        state
            .put_validator_set(current_set)
            .expect("failed putting validator set");
    }
}
