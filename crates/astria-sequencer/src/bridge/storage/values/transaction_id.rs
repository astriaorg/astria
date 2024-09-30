use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

use astria_core::primitive::v1::{
    TransactionId as DomainTransactionId,
    TRANSACTION_ID_LEN,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::hex;

use super::Value;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct TransactionId<'a>(Cow<'a, [u8; TRANSACTION_ID_LEN]>);

impl<'a> Display for TransactionId<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        hex(self.0.as_slice()).fmt(f)
    }
}

impl<'a> From<&'a DomainTransactionId> for TransactionId<'a> {
    fn from(tx_id: &'a DomainTransactionId) -> Self {
        TransactionId(Cow::Borrowed(tx_id.as_bytes()))
    }
}

impl<'a> From<TransactionId<'a>> for DomainTransactionId {
    fn from(tx_id: TransactionId<'a>) -> Self {
        DomainTransactionId::new(tx_id.0.into_owned())
    }
}

impl<'a> From<TransactionId<'a>> for crate::storage::StoredValue<'a> {
    fn from(tx_id: TransactionId<'a>) -> Self {
        crate::storage::StoredValue::Bridge(Value::TransactionId(tx_id))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for TransactionId<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value::TransactionId(tx_id)) = value else {
            bail!("bridge stored value type mismatch: expected transaction id, found {value}");
        };
        Ok(tx_id)
    }
}
