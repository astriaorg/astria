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

use crate::accounts::{
    state_ext::{
        Balance,
        Nonce,
    },
    transaction::Transaction as AccountsTransaction,
};

#[async_trait]
pub trait ActionHandler {
    fn check_stateless(&self) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()>;
}

/// Represents a sequencer chain transaction.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Transaction {
    AccountsTransaction(AccountsTransaction),
}

impl Transaction {
    pub fn new_accounts_transaction(
        to: String,
        from: String,
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
    fn check_stateless(&self) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateless(),
        }
    }

    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateful(state).await,
        }
    }

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
            "bob".to_string(),
            "alice".to_string(),
            333333,
            1,
        );
        let bytes = tx.to_bytes().unwrap();
        let tx2 = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));
    }
}
