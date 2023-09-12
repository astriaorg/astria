use anyhow::{
    ensure,
    Context as _,
    Result,
};
use ed25519_consensus::VerificationKey;
use proto::native::sequencer::v1alpha1::Address;
use tracing::instrument;

use crate::{
    authority::state_ext::{
        StateReadExt,
        StateWriteExt,
        Validator,
    },
    transaction::action_handler::ActionHandler,
};

// TODO: move to astria-proto
pub(crate) struct ValidatorUpdate {
    public_key: VerificationKey,
    voting_power: u64, // set to 0 to remove validator
}

#[async_trait::async_trait]
impl ActionHandler for ValidatorUpdate {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state.get_sudo_address().await?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        // ensure validator to be updated is in the set
        let validator_set = state.get_validator_set().await?;
        ensure!(
            validator_set
                .0
                .iter()
                .any(|v| v.public_key == self.public_key.to_bytes()),
            "validator to be updated is not in the set"
        );
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        // add validator update in non-consensus state
        // to be used in end_block
        let mut validator_updates = state
            .get_validator_updates()
            .await
            .context("failed getting validator updates")?;
        validator_updates.0.push(Validator {
            public_key: self.public_key.to_bytes(),
            voting_power: self.voting_power,
        });

        state.put_validator_updates(validator_updates)?;

        Ok(())
    }
}
