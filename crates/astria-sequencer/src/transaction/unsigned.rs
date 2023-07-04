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
    crypto::SigningKey,
    hash,
    transaction::Signed,
};

/// Represents an unsigned sequencer chain transaction.
///
/// This type wraps all the different module-specific transactions.
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Unsigned {
    AccountsTransaction(AccountsTransaction),
}

impl Unsigned {
    /// Converts the transaction into its protobuf representation.
    #[must_use]
    pub(crate) fn to_proto(&self) -> ProtoUnsignedTransaction {
        match &self {
            Unsigned::AccountsTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(ProtoAccountsTransaction(tx.to_proto())),
            },
        }
    }

    /// Signs the transaction with the given signing key.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn into_signed(self, secret_key: &SigningKey) -> Signed {
        let signature = secret_key.sign(&self.hash());
        Signed {
            transaction: self,
            signature,
            public_key: secret_key.verification_key(),
        }
    }

    pub(crate) fn hash(&self) -> Vec<u8> {
        hash(&self.to_proto().encode_length_delimited_to_vec())
    }
}

#[cfg(test)]
mod test {
    use anyhow::{
        bail,
        Context,
        Result,
    };
    use rand::rngs::OsRng;

    use super::*;
    use crate::accounts::types::{
        Address,
        Balance,
        Nonce,
        ADDRESS_LEN,
    };

    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address(arr)
    }

    impl Unsigned {
        /// Converts the protobuf representation into the corresponding `Transaction` type.
        ///
        /// # Errors
        ///
        /// - If the value is missing
        /// - If the value is not a valid transaction type (ie. does not correspond to any
        ///   component)
        fn try_from_proto(proto: &ProtoUnsignedTransaction) -> Result<Self> {
            let Some(ref value) = proto.value else {
                bail!("invalid unsigned transaction; missing value");
            };

            Ok(match value {
                ProtoAccountsTransaction(tx) => Self::AccountsTransaction(
                    AccountsTransaction::try_from_proto(tx)
                        .context("failed to convert proto to AccountsTransaction")?,
                ),
            })
        }
    }

    #[test]
    fn test_unsigned_transaction() {
        let tx = Unsigned::AccountsTransaction(AccountsTransaction::new(
            address_from_hex_string(BOB_ADDRESS),
            Balance::from(333_333),
            Nonce::from(1),
        ));
        let bytes = tx.to_proto().encode_length_delimited_to_vec();
        let proto = ProtoUnsignedTransaction::decode_length_delimited(bytes.as_slice()).unwrap();
        let tx2 = Unsigned::try_from_proto(&proto).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));

        let secret_key: SigningKey = SigningKey::new(OsRng);
        let signed = tx.into_signed(&secret_key);
        signed.verify_signature().unwrap();
    }
}
