use std::borrow::Cow;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct BlockHash<'a>(Cow<'a, [u8; 32]>);

impl<'a> From<&'a [u8; 32]> for BlockHash<'a> {
    fn from(block_hash: &'a [u8; 32]) -> Self {
        BlockHash(Cow::Borrowed(block_hash))
    }
}

impl<'a> From<BlockHash<'a>> for [u8; 32] {
    fn from(block_hash: BlockHash<'a>) -> Self {
        block_hash.0.into_owned()
    }
}

impl<'a> TryFrom<StoredValue<'a>> for BlockHash<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::BlockHash(block_hash) = value else {
            return Err(super::type_mismatch("block hash", &value));
        };
        Ok(block_hash)
    }
}
