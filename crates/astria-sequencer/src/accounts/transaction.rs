use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_proto::sequencer::v1::Transfer as ProtoAccountsTransaction;
use serde::{
    Deserialize,
    Serialize,
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
        },
    },
    transaction::action_handler::ActionHandler,
};

/// Represents a value-transfer transaction.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Transfer {
    to: Address,
    amount: Balance,
}

impl Transfer {
    #[allow(dead_code)]
    pub(crate) fn new(to: Address, amount: Balance) -> Self {
        Self {
            to,
            amount,
        }
    }

    pub(crate) fn to_proto(&self) -> ProtoAccountsTransaction {
        ProtoAccountsTransaction {
            to: self.to.as_bytes().to_vec(),
            amount: Some(self.amount.into()),
            // nonce: self.nonce.into(),
        }
    }

    pub(crate) fn try_from_proto(proto: &ProtoAccountsTransaction) -> Result<Self> {
        Ok(Self {
            to: Address::try_from(proto.to.as_ref() as &[u8])?,
            amount: proto
                .amount
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing amount"))?
                .into(),
        })
    }
}

#[async_trait::async_trait]
impl ActionHandler for Transfer {
    fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: &Address,
    ) -> Result<()> {
        let curr_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        ensure!(curr_balance >= self.amount, "insufficient funds");

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount.into_inner(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: &Address) -> Result<()> {
        let from_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        let to_balance = state
            .get_account_balance(&self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(from, from_balance - self.amount)
            .context("failed updating `from` account balance")?;
        state
            .put_account_balance(&self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;
        Ok(())
    }
}
