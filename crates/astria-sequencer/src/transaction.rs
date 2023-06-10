use anyhow::Result;
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

use crate::accounts::{
    transaction::Transaction as AccountsTransaction,
    types::{
        Address,
        Balance,
        Nonce,
    },
};

#[async_trait]
pub trait ActionHandler {
    fn check_stateless(&self) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()>;
}

/// Represents a sequencer chain transaction.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Transaction {
    AccountsTransaction(AccountsTransaction),
}

impl Transaction {
    pub fn new_accounts_transaction(
        to: Address,
        from: Address,
        amount: Balance,
        nonce: Nonce,
    ) -> Self {
        Self::AccountsTransaction(AccountsTransaction::new(to, from, amount, nonce))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tx = serde_json::from_slice(bytes)?;
        Ok(tx)
    }
}

#[async_trait]
impl ActionHandler for Transaction {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateless(),
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateful(state).await,
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.execute(state).await,
        }
    }
}

#[cfg(test)]
mod test {
    use hex;

    use super::*;

    #[test]
    fn test_transaction() {
        let tx = Transaction::new_accounts_transaction(
            Address::from("bob"),
            Address::from("alice"),
            Balance::from(333333),
            Nonce::from(1),
        );
        let bytes = tx.to_bytes().unwrap();
        let tx2 = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));
    }
}
