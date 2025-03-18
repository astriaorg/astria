use std::sync::Arc;

use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::action::ValidatorUpdate,
};
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};
use tracing::{
    instrument,
    Level,
};

use super::{
    StateReadExt,
    StateWriteExt,
    ValidatorSet,
};
use crate::component::{
    Component,
    PrepareStateInfo,
};

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

    #[instrument(name = "AuthorityComponent::init_chain", skip_all, err)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // set sudo key and initial validator set
        state
            .put_sudo_address(app_state.authority_sudo_address)
            .wrap_err("failed to set sudo key")?;
        let genesis_validators = app_state.genesis_validators.clone();
        state
            .put_validator_set(ValidatorSet::new_from_updates(genesis_validators))
            .wrap_err("failed to set validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::begin_block", skip_all, err(level = Level::WARN))]
    async fn begin_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        prepare_state_info: &PrepareStateInfo,
    ) -> Result<()> {
        let mut current_set = state
            .get_validator_set()
            .await
            .wrap_err("failed getting validator set")?;

        for misbehaviour in &prepare_state_info.byzantine_validators {
            current_set.remove(&misbehaviour.validator.address);
        }

        let state = Arc::get_mut(state)
            .ok_or_eyre("must only have one reference to the state; this is a bug")?;
        state
            .put_validator_set(current_set)
            .wrap_err("failed putting validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::end_block", skip_all, err(level = Level::WARN))]
    async fn end_block<S: StateWriteExt + StateReadExt + 'static>(
        state: &mut Arc<S>,
    ) -> Result<()> {
        // update validator set
        let validator_updates = state
            .get_validator_updates()
            .await
            .wrap_err("failed getting validator updates")?;

        let mut current_set = state
            .get_validator_set()
            .await
            .wrap_err("failed getting validator set")?;
        current_set.apply_updates(validator_updates);

        let state = Arc::get_mut(state)
            .ok_or_eyre("must only have one reference to the state; this is a bug")?;
        state
            .put_validator_set(current_set)
            .wrap_err("failed putting validator set")?;
        Ok(())
    }
}
