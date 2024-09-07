use std::borrow::Cow;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct TransactionHash<'a>(Cow<'a, [u8; 32]>);

impl<'a> From<&'a [u8; 32]> for TransactionHash<'a> {
    fn from(tx_hash: &'a [u8; 32]) -> Self {
        TransactionHash(Cow::Borrowed(tx_hash))
    }
}

impl<'a> From<TransactionHash<'a>> for [u8; 32] {
    fn from(tx_hash: TransactionHash<'a>) -> Self {
        tx_hash.0.into_owned()
    }
}

impl<'a> TryFrom<StoredValue<'a>> for TransactionHash<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::TransactionHash(tx_hash) = value else {
            return Err(super::type_mismatch("transaction hash", &value));
        };
        Ok(tx_hash)
    }
}
