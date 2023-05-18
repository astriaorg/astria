use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

use crate::accounts::{state_ext::{StateReadExt, StateWriteExt}};

pub type Balance = u64; // might need to be larger
pub type Nonce = u32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transaction {
    to: String,
    from: String,
    amount: Balance, 
    nonce: Nonce,
}

impl Transaction {
    pub fn new(to: String, from: String, amount: Balance, nonce: Nonce) -> Self {
        Self {
            to,
            from,
            amount,
            nonce,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tx = serde_json::from_slice(bytes)?;
        Ok(tx)
    }

    pub fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    pub async fn check_stateful<S: StateReadExt + 'static>(&self, state: &S) -> Result<()> {
        let (balance, nonce) = state
            .get_account_state(&self.from)
            .await?;

        // TODO: do nonces start at 0 or 1? this assumes an account's first tx has nonce 1.
        if nonce <= self.nonce {
            anyhow::bail!("invalid nonce");
        }

        if balance < self.amount {
            anyhow::bail!("insufficient funds");
        }

        Ok(())
    }

    pub async fn execute<S: StateWriteExt>(&self, state: &mut S) -> Result<()> {
        let (from_balance, from_nonce) = state
            .get_account_state(&self.from)
            .await?;
        let (to_balance, to_nonce) = state
            .get_account_state(&self.to)
            .await?;

        state.put_account_state(&self.from, from_balance - self.amount, from_nonce + 1);
        state.put_account_state(&self.to, to_balance + self.amount, to_nonce);
        Ok(())
    }
}
