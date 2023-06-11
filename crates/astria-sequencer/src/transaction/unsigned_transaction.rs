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
    pub fn new_accounts_transaction(to: Address, amount: Balance, nonce: Nonce) -> Self {
        Self::AccountsTransaction(AccountsTransaction::new(to, amount, nonce))
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

    pub(crate) fn hash(&self) -> Result<Vec<u8>> {
        let bytes = self.to_bytes().context("failed to serialize transaction")?;
        Ok(hash(&bytes))
    }
}

#[cfg(test)]
mod test {
    use hex;

    use super::*;
    use crate::app::BOB_ADDRESS;

    #[test]
    fn test_transaction() {
        let tx = UnsignedTransaction::new_accounts_transaction(
            Address::unsafe_from_hex_string(BOB_ADDRESS),
            Balance::from(333333),
            Nonce::from(1),
        );
        let bytes = tx.to_bytes().unwrap();
        let tx2 = UnsignedTransaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));
    }
}
