use anyhow::{
    ensure,
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use tracing::instrument;

use crate::{
    accounts::{
        state_ext::{
            StateReadExt,
            StateWriteExt,
        },
        types::{
            Address,
            Balance,
            Nonce,
        },
    },
    transaction::ActionHandler,
};

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Transaction {
    pub(crate) to: Address,
    pub(crate) from: Address,
    pub(crate) amount: Balance,
    pub(crate) nonce: Nonce,
}

#[async_trait]
impl ActionHandler for Transaction {
    fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(&self, state: &S) -> Result<()> {
        let curr_nonce = state
            .get_account_nonce(&self.from)
            .await
            .context("failed getting `from` account nonce")?;

        // TODO: do nonces start at 0 or 1? this assumes an account's first tx has nonce 1.
        ensure!(curr_nonce < self.nonce, "invalid nonce",);

        let curr_balance = state
            .get_account_balance(&self.from)
            .await
            .context("failed getting `from` account balance")?;
        ensure!(curr_balance >= self.amount, "insufficient funds",);

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.as_ref(),
            from = self.from.as_ref(),
            amount = self.amount.into_inner(),
            nonce = self.nonce.into_inner(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S) -> Result<()> {
        let from_balance = state
            .get_account_balance(&self.from)
            .await
            .context("failed getting `from` account balance")?;
        let from_nonce = state
            .get_account_nonce(&self.from)
            .await
            .context("failed getting `from` nonce")?;
        let to_balance = state
            .get_account_balance(&self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(&self.from, from_balance - self.amount)
            .context("failed updating `from` account balance")?;
        state
            .put_account_nonce(&self.from, from_nonce + Nonce::from(1))
            .context("failed updating `from` nonce")?;
        state
            .put_account_balance(&self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;
        Ok(())
    }
}
