use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::action::ValidatorUpdate,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use tracing::instrument;

use super::{
    StateWriteExt,
    ValidatorSet,
};
use crate::genesis::Genesis;

#[derive(Default)]
pub(crate) struct AuthorityComponent;

#[derive(Debug)]
pub(crate) struct AuthorityComponentAppState {
    pub(crate) authority_sudo_address: Address,
    pub(crate) genesis_validators: Vec<ValidatorUpdate>,
}

#[async_trait::async_trait]
impl Genesis for AuthorityComponent {
    type AppState = AuthorityComponentAppState;

    #[instrument(name = "AuthorityComponent::init_chain", skip_all)]
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
}
