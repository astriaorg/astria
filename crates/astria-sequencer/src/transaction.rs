use anyhow::{
    anyhow,
    ensure,
    Context as _,
    Result,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use serde::{
    Deserialize,
    Serialize,
};
use sha2::Digest as _;
use tracing::instrument;

use crate::{
    accounts::{
        transaction::Transaction as AccountsTransaction,
        types::{
            Address,
            Balance,
            Nonce,
        },
    },
    crypto::{
        Keypair,
        PublicKey,
        Signature,
        Signer,
        Verifier,
    },
};

#[async_trait]
pub trait ActionHandler {
    fn check_stateless(&self) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()>;
}

/// Represents the sha256 hash of an encoded transaction.
pub struct TransactionHash([u8; 32]);

impl TryFrom<&[u8]> for TransactionHash {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        ensure!(value.len() == 32, "invalid slice length; must be 32");

        let buf: [u8; 32] = value.try_into()?;
        Ok(TransactionHash(buf))
    }
}

impl TransactionHash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
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

    pub fn hash(&self) -> Result<TransactionHash> {
        Ok(TransactionHash(
            hash(&self.to_bytes()?)
                .try_into()
                .map_err(|_| anyhow!("failed to turn hash into 32 bytes"))?,
        ))
    }

    pub fn sign(self, keypair: &Keypair) -> Result<SignedTransaction> {
        let hash = self.to_bytes().context("failed hashing namespace data")?;
        let signature = keypair.sign(&hash);
        Ok(SignedTransaction {
            transaction: self,
            signature,
            public_key: keypair.public,
        })
    }
}

fn hash(s: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(s);
    hasher.finalize().to_vec()
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

pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Signature,
    pub public_key: PublicKey,
}

impl SignedTransaction {
    pub fn verify(&self) -> Result<()> {
        self.public_key
            .verify(self.transaction.hash()?.as_bytes(), &self.signature)
            .context("failed to verify transaction signature")
    }

    pub fn from(&self) -> Address {
        todo!()
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
