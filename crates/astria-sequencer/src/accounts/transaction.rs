use anyhow::{
    ensure,
    Result,
};
use astria_proto::sequencer::v1::AccountsTransaction as ProtoAccountsTransaction;
use async_trait::async_trait;
use serde::{
    Deserialize,
    Serialize,
};

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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transaction {
    to: Address,
    from: Address, // TODO: remove in favour of signing pubkey
    amount: Balance,
    nonce: Nonce,
}

impl Transaction {
    pub fn new(to: Address, from: Address, amount: Balance, nonce: Nonce) -> Self {
        Self {
            to,
            from,
            amount,
            nonce,
        }
    }

    pub fn to_proto(&self) -> ProtoAccountsTransaction {
        ProtoAccountsTransaction {
            to: self.to.as_bytes().to_vec(),
            from: self.from.as_bytes().to_vec(),
            amount: Some(self.amount.to_proto()),
            nonce: self.nonce.into(),
        }
    }

    pub fn from_proto(proto: &ProtoAccountsTransaction) -> Result<Self> {
        Ok(Self {
            to: Address::try_from(proto.to.as_ref() as &[u8])?,
            from: Address::try_from(proto.from.as_ref() as &[u8])?,
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

#[async_trait]
impl ActionHandler for Transaction {
    fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(&self, state: &S) -> Result<()> {
        let curr_nonce = state.get_account_nonce(&self.from).await?;

        // TODO: do nonces start at 0 or 1? this assumes an account's first tx has nonce 1.
        ensure!(curr_nonce < self.nonce, "invalid nonce",);

        let curr_balance = state.get_account_balance(&self.from).await?;
        ensure!(curr_balance >= self.amount, "insufficient funds",);

        Ok(())
    }

    async fn execute<S: StateWriteExt>(&self, state: &mut S) -> Result<()> {
        let from_balance = state.get_account_balance(&self.from).await?;
        let from_nonce = state.get_account_nonce(&self.from).await?;
        let to_balance = state.get_account_balance(&self.to).await?;
        state.put_account_balance(&self.from, from_balance - self.amount)?;
        state.put_account_nonce(&self.from, from_nonce + Nonce::from(1))?;
        state.put_account_balance(&self.to, to_balance + self.amount)?;
        Ok(())
    }
}
