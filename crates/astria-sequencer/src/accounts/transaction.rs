use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_proto::sequencer::v1::AccountsTransaction as ProtoAccountsTransaction;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

use crate::accounts::{
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    types::{
        Address,
        Balance,
        Nonce,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transaction {
    to: Address,
    amount: Balance,
    nonce: Nonce,
}

impl Transaction {
    pub fn new(to: Address, amount: Balance, nonce: Nonce) -> Self {
        Self {
            to,
            amount,
            nonce,
        }
    }

    pub fn to_proto(&self) -> ProtoAccountsTransaction {
        ProtoAccountsTransaction {
            to: self.to.as_bytes().to_vec(),
            amount: Some(self.amount.to_proto()),
            nonce: self.nonce.into(),
        }
    }

    pub fn from_proto(proto: &ProtoAccountsTransaction) -> Result<Self> {
        Ok(Self {
            to: Address::try_from(proto.to.as_ref() as &[u8])?,
            amount: proto
                .amount
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing amount"))?
                .into(),
            nonce: Nonce::from(proto.nonce),
        })
    }
}

impl TryFrom<ProtoAccountsTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(proto: ProtoAccountsTransaction) -> Result<Self> {
        Self::from_proto(&proto)
    }
}

impl TryFrom<&ProtoAccountsTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(proto: &ProtoAccountsTransaction) -> Result<Self> {
        Self::from_proto(proto)
    }
}

impl From<Transaction> for ProtoAccountsTransaction {
    fn from(tx: Transaction) -> Self {
        tx.to_proto()
    }
}

impl From<&Transaction> for ProtoAccountsTransaction {
    fn from(tx: &Transaction) -> Self {
        tx.to_proto()
    }
}

impl Transaction {
    pub fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    pub async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: &Address,
    ) -> Result<()> {
        let curr_nonce = state.get_account_nonce(from).await?;

        // TODO: do nonces start at 0 or 1? this assumes an account's first tx has nonce 1.
        ensure!(curr_nonce < self.nonce, "invalid nonce",);

        let curr_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        ensure!(curr_balance >= self.amount, "insufficient funds",);

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount.into_inner(),
            nonce = self.nonce.into_inner(),
        )
    )]
    pub async fn execute<S: StateWriteExt>(&self, state: &mut S, from: &Address) -> Result<()> {
        let from_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        let from_nonce = state
            .get_account_nonce(from)
            .await
            .context("failed getting `from` nonce")?;
        let to_balance = state
            .get_account_balance(&self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(from, from_balance - self.amount)
            .context("failed updating `from` account balance")?;
        state
            .put_account_nonce(from, from_nonce + Nonce::from(1))
            .context("failed updating `from` nonce")?;
        state
            .put_account_balance(&self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;
        Ok(())
    }
}
