use crate::account::types::Balance;
use astria_proto::{
    native::sequencer::v1alpha1::Address,
};

/// Represents a request for funds from the faucet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    to: Address,
    amount: Balance,
}

#[async_trait::async_trait]
impl ActionHandler for Request {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        _: Address,
    ) -> Result<()> {
        // TODO: check that `to` hasn't exceeded their daily limit
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount.into_inner(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
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
