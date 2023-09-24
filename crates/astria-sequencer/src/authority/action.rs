use anyhow::{
    ensure,
    Context as _,
    Result,
};
use proto::native::sequencer::v1alpha1::Address;
use tracing::instrument;

use crate::{
    authority::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for tendermint::validator::Update {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        // add validator update in non-consensus state to be used in end_block
        let mut validator_updates = state
            .get_validator_updates()
            .await
            .context("failed getting validator updates from state")?;
        validator_updates.push_update(self.clone());
        state
            .put_validator_updates(validator_updates)
            .context("failed to put validator updates in state")?;
        Ok(())
    }
}
