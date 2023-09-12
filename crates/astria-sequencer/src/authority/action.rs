use anyhow::Result;
use ed25519_consensus::VerificationKey;
use proto::native::sequencer::v1alpha1::Address;
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
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
        // ensure validator to be updated is in the set
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        // add validator update in non-consensus state
        // to be used in end_block
        Ok(())
    }
}
