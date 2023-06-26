use astria_proto::sequencer::v1::{
    unsigned_transaction::Value::AccountsTransaction as ProtoAccountsTransaction,
    UnsignedTransaction as ProtoUnsignedTransaction,
};
use prost::Message as _;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::transaction::Transaction as AccountsTransaction,
    hash,
};

/// Represents an unsigned sequencer chain transaction.
/// This type wraps all the different module-specific transactions.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Transaction {
    AccountsTransaction(AccountsTransaction),
}

impl Transaction {
    /// Attempts to encode the unsigned transaction into bytes.
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        match &self {
            Transaction::AccountsTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(ProtoAccountsTransaction(tx.to_proto())),
            },
        }
        .encode_length_delimited_to_vec()
    }

    pub(crate) fn hash(&self) -> Vec<u8> {
        hash(&self.to_vec())
    }
}

#[cfg(test)]
mod test {
    use anyhow::{
        bail,
        Context as _,
        Result,
    };
    use rand::rngs::OsRng;

    use super::*;
    use crate::{
        accounts::types::{
            Address,
            Balance,
            Nonce,
            ADDRESS_LEN,
        },
        crypto::SigningKey,
        transaction::signed::Transaction as SignedTransaction,
    };

    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address::from_array(arr)
    }

    impl Transaction {
        /// Attempts to decode an unsigned transaction from the given bytes.
        ///
        /// # Errors
        ///
        /// - If the bytes cannot be decoded into the prost-generated `UnsignedTransaction` type
        /// - If the value is missing
        /// - If the value is not a valid transaction type (ie. does not correspond to any
        ///   component)
        pub fn try_from_slice(bytes: &[u8]) -> Result<Self> {
            let proto = ProtoUnsignedTransaction::decode_length_delimited(bytes)
                .context("failed to decode unsigned transaction")?;
            let Some(value) = proto.value else {
            bail!("invalid unsigned transaction; missing value");
        };

            Ok(match value {
                ProtoAccountsTransaction(tx) => {
                    Self::AccountsTransaction(AccountsTransaction::try_from_proto(&tx)?)
                }
            })
        }

        /// Signs the transaction with the given keypair.
        #[must_use]
        pub fn sign(self, secret_key: &SigningKey) -> SignedTransaction {
            let signature = secret_key.sign(&self.hash());
            SignedTransaction {
                transaction: self,
                signature,
                public_key: secret_key.verification_key(),
            }
        }
    }

    #[test]
    fn test_unsigned_transaction() {
        let tx = Transaction::AccountsTransaction(AccountsTransaction::new(
            address_from_hex_string(BOB_ADDRESS),
            Balance::from(333_333),
            Nonce::from(1),
        ));
        let bytes = tx.to_vec();
        let tx2 = Transaction::try_from_slice(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));

        let secret_key: SigningKey = SigningKey::new(OsRng);
        let signed = tx.sign(&secret_key);
        signed.verify_signature().unwrap();
    }
}
