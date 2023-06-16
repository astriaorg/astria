use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::UnsignedTransaction as ProtoUnsignedTransaction;
use prost::Message as _;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::transaction::Transaction as AccountsTransaction,
    crypto::{
        Keypair,
        Signer,
    },
    hash,
    transaction::SignedTransaction,
};

/// Represents an unsigned sequencer chain transaction.
/// This type wraps all the different module-specific transactions.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UnsignedTransaction {
    AccountsTransaction(AccountsTransaction),
}

impl UnsignedTransaction {
    /// Attempts to encode the unsigned transaction into bytes.
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        match &self {
            UnsignedTransaction::AccountsTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(
                    astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction(
                        tx.to_proto(),
                    ),
                ),
            },
        }
        .encode_length_delimited_to_vec()
    }

    /// Attempts to decode an unsigned transaction from the given bytes.
    ///
    /// # Errors
    ///
    /// - If the bytes cannot be decoded into the prost-generated `UnsignedTransaction` type
    /// - If the value is missing
    /// - If the value is not a valid transaction type (ie. does not correspond to any component)
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self> {
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

    /// Signs the transaction with the given keypair.
    #[must_use]
    pub fn sign(self, keypair: &Keypair) -> SignedTransaction {
        let signature = keypair.sign(&self.hash());
        SignedTransaction {
            transaction: self,
            signature,
            public_key: keypair.public,
        }
    }

    pub(crate) fn hash(&self) -> Vec<u8> {
        hash(&self.to_vec())
    }
}

#[cfg(test)]
mod test {
    use hex;

    use super::*;
    use crate::accounts::types::{
        Address,
        Balance,
        Nonce,
    };

    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    #[test]
    fn test_transaction() {
        let tx = UnsignedTransaction::AccountsTransaction(AccountsTransaction::new(
            Address::unsafe_from_hex_string(BOB_ADDRESS),
            Balance::from(333_333),
            Nonce::from(1),
        ));
        let bytes = tx.to_vec();
        let tx2 = UnsignedTransaction::try_from_slice(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));
    }
}
