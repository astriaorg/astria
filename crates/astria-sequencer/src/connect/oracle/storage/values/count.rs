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
pub(in crate::connect::oracle) struct Count(u64);

impl From<u64> for Count {
    fn from(count: u64) -> Self {
        Count(count)
    }
}

impl From<Count> for u64 {
    fn from(count: Count) -> Self {
        count.0
    }
}

impl From<Count> for crate::storage::StoredValue<'_> {
    fn from(count: Count) -> Self {
        crate::storage::StoredValue::ConnectOracle(Value(ValueImpl::Count(count)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Count {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::ConnectOracle(Value(ValueImpl::Count(count))) = value
        else {
            bail!("connect oracle stored value type mismatch: expected count, found {value:?}");
        };
        Ok(count)
    }
}
