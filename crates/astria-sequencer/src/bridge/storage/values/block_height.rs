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

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::bridge) struct BlockHeight(u64);

impl Display for BlockHeight {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u64> for BlockHeight {
    fn from(block_height: u64) -> Self {
        BlockHeight(block_height)
    }
}

impl From<BlockHeight> for u64 {
    fn from(block_height: BlockHeight) -> Self {
        block_height.0
    }
}

impl<'a> From<BlockHeight> for crate::storage::StoredValue<'a> {
    fn from(block_height: BlockHeight) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::BlockHeight(block_height)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BlockHeight {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::BlockHeight(block_height))) =
            value
        else {
            bail!("bridge stored value type mismatch: expected block height, found {value:?}");
        };
        Ok(block_height)
    }
}
