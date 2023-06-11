use anyhow::{
    anyhow,
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::{
    SignedTransaction as ProtoSignedTransaction,
    UnsignedTransaction as ProtoUnsignedTransaction,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use prost::Message as _;
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

/// Represents an unsigned sequencer chain transaction.
/// This type wraps all the different module-specific transactions.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UnsignedTransaction {
    AccountsTransaction(AccountsTransaction),
}

impl UnsignedTransaction {
    pub fn new_accounts_transaction(
        to: Address,
        from: Address,
        amount: Balance,
        nonce: Nonce,
    ) -> Self {
        Self::AccountsTransaction(AccountsTransaction::new(to, from, amount, nonce))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(match &self {
            UnsignedTransaction::AccountsTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(
                    astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction(
                        tx.to_proto(),
                    ),
                ),
            },
        }
        .encode_length_delimited_to_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let proto = ProtoUnsignedTransaction::decode_length_delimited(bytes)
            .context("failed to decode unsigned transaction")?;
        let Some(value) = proto.value else {
            bail!("invalid unsigned transaction; missing value");
        };

        Ok(match value {
            astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction(tx) => {
                Self::AccountsTransaction(AccountsTransaction::from_proto(&tx)?)
            }
        })
    }

    pub fn sign(self, keypair: &Keypair) -> Result<SignedTransaction> {
        let signature = keypair.sign(&self.hash()?);
        Ok(SignedTransaction {
            transaction: self,
            signature,
            public_key: keypair.public,
        })
    }

    fn hash(&self) -> Result<Vec<u8>> {
        let bytes = self.to_bytes().context("failed to serialize transaction")?;
        Ok(hash(&bytes))
    }
}

fn hash(s: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(s);
    hasher.finalize().to_vec()
}

#[async_trait]
impl ActionHandler for SignedTransaction {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        self.verify_signature()?;
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => tx.check_stateless(),
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => tx.check_stateful(state).await,
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => tx.execute(state).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub signature: Signature,
    pub public_key: PublicKey,
    pub transaction: UnsignedTransaction,
}

impl SignedTransaction {
    pub fn verify_signature(&self) -> Result<()> {
        self.public_key
            .verify(&self.transaction.hash()?, &self.signature)
            .context("failed to verify transaction signature")
    }

    pub fn from_address(&self) -> Address {
        todo!()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let proto = ProtoSignedTransaction {
            transaction: Some(match &self.transaction {
                UnsignedTransaction::AccountsTransaction(tx) => {
                    astria_proto::sequencer::v1::UnsignedTransaction {
                        value: Some(astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction (
                            tx.to_proto()
                        )),
                    }
                }
            }),
            signature: self.signature.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
        };

        let bytes = proto.encode_length_delimited_to_vec();
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let proto_tx: ProtoSignedTransaction =
            ProtoSignedTransaction::decode_length_delimited(bytes)?;
        let Some(proto_transaction) = proto_tx.transaction else {
            bail!("transaction is missing");
        };

        let Some(value) = proto_transaction.value else {
            bail!("unsigned transaction value missing")
        };

        let transaction = match value {
            astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction(tx) => {
                UnsignedTransaction::AccountsTransaction(AccountsTransaction::from_proto(&tx)?)
            }
        };
        let signed_tx = SignedTransaction {
            transaction,
            signature: Signature::from_bytes(&proto_tx.signature)?,
            public_key: PublicKey::from_bytes(&proto_tx.public_key)?,
        };
        Ok(signed_tx)
    }

    pub fn hash(&self) -> Result<TransactionHash> {
        Ok(TransactionHash(
            hash(&self.to_bytes()?)
                .try_into()
                .map_err(|_| anyhow!("failed to turn hash into 32 bytes"))?,
        ))
    }
}

#[cfg(test)]
mod test {
    use hex;

    use super::*;
    use crate::app::{
        ALICE_ADDRESS,
        BOB_ADDRESS,
    };

    #[test]
    fn test_transaction() {
        let tx = UnsignedTransaction::new_accounts_transaction(
            Address::unsafe_from_hex_string(BOB_ADDRESS),
            Address::unsafe_from_hex_string(ALICE_ADDRESS),
            Balance::from(333333),
            Nonce::from(1),
        );
        let bytes = tx.to_bytes().unwrap();
        let tx2 = UnsignedTransaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));
    }
}
