use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::ValidatorUpdate,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use super::{
    StateReadExt,
    StateWriteExt,
    ValidatorSet,
};
use crate::component::Component;

#[derive(Default)]
pub(crate) struct AuthorityComponent;

#[derive(Debug)]
pub(crate) struct AuthorityComponentAppState {
    pub(crate) authority_sudo_address: Address,
    pub(crate) genesis_validators: Vec<ValidatorUpdate>,
}

#[async_trait::async_trait]
impl Component for AuthorityComponent {
    type AppState = AuthorityComponentAppState;

    #[instrument(name = "AuthorityComponent::init_chain", skip_all)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // set sudo key and initial validator set
        state
            .put_sudo_address(app_state.authority_sudo_address)
            .context("failed to set sudo key")?;
        let genesis_validators = app_state.genesis_validators.clone();
        state
            .put_validator_set(ValidatorSet::new_from_updates(genesis_validators))
            .context("failed to set validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::begin_block", skip_all)]
    async fn begin_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        begin_block: &BeginBlock,
    ) -> Result<()> {
        let mut current_set = state
            .get_validator_set()
            .await
            .context("failed getting validator set")?;

        for misbehaviour in &begin_block.byzantine_validators {
            current_set.remove(&misbehaviour.validator.address);
        }

        let state = Arc::get_mut(state)
            .context("must only have one reference to the state; this is a bug")?;
        state
            .put_validator_set(current_set)
            .context("failed putting validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::end_block", skip_all)]
    async fn end_block<S: StateWriteExt + StateReadExt + 'static>(
        state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        // update validator set
        let validator_updates = state
            .get_validator_updates()
            .await
            .context("failed getting validator updates")?;

        let mut current_set = state
            .get_validator_set()
            .await
            .context("failed getting validator set")?;
        current_set.apply_updates(validator_updates);

        let state = Arc::get_mut(state)
            .context("must only have one reference to the state; this is a bug")?;
        state
            .put_validator_set(current_set)
            .context("failed putting validator set")?;
        Ok(())
    }
}
