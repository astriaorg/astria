use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::{
    Address,
    TransferAction,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

/// Fee charged for a `Transfer` action.
pub(crate) const TRANSFER_FEE: u128 = 12;

#[async_trait::async_trait]
impl ActionHandler for TransferAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        let curr_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        ensure!(
            curr_balance >= self.amount + TRANSFER_FEE,
            "insufficient funds"
        );

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount,
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let from_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        let to_balance = state
            .get_account_balance(self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(from, from_balance - (self.amount + TRANSFER_FEE))
            .context("failed updating `from` account balance")?;
        state
            .put_account_balance(self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;
        Ok(())
    }
}
