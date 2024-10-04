use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct BlockHash<'a>(Cow<'a, [u8; 32]>);

impl<'a> Debug for BlockHash<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

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

impl<'a> From<BlockHash<'a>> for crate::storage::StoredValue<'a> {
    fn from(block_hash: BlockHash<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::BlockHash(block_hash)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BlockHash<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::BlockHash(block_hash))) = value
        else {
            bail!("grpc stored value type mismatch: expected block hash, found {value:?}");
        };
        Ok(block_hash)
    }
}
