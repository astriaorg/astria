use anyhow::{
    anyhow,
    Result,
};
use penumbra_storage::{
    StateRead,
    StateWrite,
};

use crate::accounts::transaction::{
    Balance,
    Nonce,
    Transaction as AccountsTransaction,
};

/// Represents a sequencer chain transaction.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        let bytes = match self {
            Self::AccountsTransaction(tx) => {
                let mut bytes = vec![0u8];
                bytes.append(&mut tx.to_bytes()?);
                bytes
            }
        };
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(anyhow!("invalid transaction, bytes length is 0"));
        }

        match bytes[0] {
            0 => Ok(Self::AccountsTransaction(AccountsTransaction::from_bytes(
                &bytes[1..],
            )?)),
            _ => Err(anyhow!("invalid transaction type")),
        }
    }

    pub fn check_stateless(&self) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateless(),
        }
    }

    pub async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateful(state).await,
        }
    }

    pub async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
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
