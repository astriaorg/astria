use std::borrow::Cow;

use astria_core::primitive::v1::{
    TransactionId,
    TRANSACTION_ID_LEN,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::app) struct ExecutedTransactionHashes<'a>(Vec<Cow<'a, [u8; TRANSACTION_ID_LEN]>>);

impl<'a> From<&'a Vec<TransactionId>> for ExecutedTransactionHashes<'a> {
    fn from(executed_transaction_hashes: &'a Vec<TransactionId>) -> Self {
        ExecutedTransactionHashes(
            executed_transaction_hashes
                .iter()
                .map(|id| Cow::Owned(id.get()))
                .collect(),
        )
    }
}

impl<'a> From<ExecutedTransactionHashes<'a>> for Vec<TransactionId> {
    fn from(executed_transaction_hashes: ExecutedTransactionHashes<'a>) -> Self {
        executed_transaction_hashes
            .0
            .iter()
            .map(|id| TransactionId::new(**id))
            .collect()
    }
}

impl<'a> From<ExecutedTransactionHashes<'a>> for crate::storage::StoredValue<'a> {
    fn from(executed_transaction_hashes: ExecutedTransactionHashes<'a>) -> Self {
        crate::storage::StoredValue::App(Value(ValueImpl::ExecutedTransactionHashes(
            executed_transaction_hashes,
        )))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ExecutedTransactionHashes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::App(Value(ValueImpl::ExecutedTransactionHashes(
            executed_transaction_hashes,
        ))) = value
        else {
            bail!(
                "app stored value type mismatch: expected executed transaction hashes, found \
                 {value:?}"
            );
        };
        Ok(executed_transaction_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_serialization_round_trip() {
        let executed_transaction_hashes = vec![TransactionId::new([0; TRANSACTION_ID_LEN])];
        let executed_transaction_hashes =
            ExecutedTransactionHashes::from(&executed_transaction_hashes);
        let serialized = borsh::to_vec(&executed_transaction_hashes).unwrap();
        let deserialized: ExecutedTransactionHashes = borsh::from_slice(&serialized).unwrap();
        assert_eq!(executed_transaction_hashes.0, deserialized.0);
    }
}
