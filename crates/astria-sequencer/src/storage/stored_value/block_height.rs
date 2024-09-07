use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct BlockHeight(u64);

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

impl<'a> TryFrom<StoredValue<'a>> for BlockHeight {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::BlockHeight(block_height) = value else {
            return Err(super::type_mismatch("block height", &value));
        };
        Ok(block_height)
    }
}
