use anyhow::{
    ensure,
    Context as _,
    Result,
};
use proto::native::sequencer::{
    asset,
    v1alpha1::{
        Address,
        MintAction,
    },
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt as AccountStateReadExt,
        StateWriteExt as AccountStateWriteExt,
    },
    authority::state_ext::StateReadExt as AuthorityStateReadExt,
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for MintAction {
    async fn check_stateful<S: AuthorityStateReadExt>(
        &self,
        state: &S,
        from: Address,
        _fee_asset: &asset::Id,
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
    async fn execute<S: AccountStateWriteExt + AccountStateReadExt>(
        &self,
        state: &mut S,
        _: Address,
        _: &asset::Id,
    ) -> Result<()> {
        let to_balance = state
            .get_account_balance(self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;
        Ok(())
    }
}
