use std::fmt::{
    self,
    Display,
    Formatter,
};

use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum Value {
    Fee(Fee),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Fee(fee) => write!(f, "fee {}", fee.0),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Fee(u128);

impl From<u128> for Fee {
    fn from(fee: u128) -> Self {
        Fee(fee)
    }
}

impl From<Fee> for u128 {
    fn from(fee: Fee) -> Self {
        fee.0
    }
}

impl<'a> From<Fee> for crate::storage::StoredValue<'a> {
    fn from(fee: Fee) -> Self {
        crate::storage::StoredValue::Sequence(Value::Fee(fee))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Fee {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Sequence(Value::Fee(fee)) = value else {
            bail!("sequence stored value type mismatch: expected fee, found {value}");
        };
        Ok(fee)
    }
}
