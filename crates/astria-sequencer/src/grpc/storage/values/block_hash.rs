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

// NOTE(janis): Is it confusing that the display impl at the service level is hex,
// while here it's base64? This probably makes sense because storage is closer to
// the wire format, which itself followes the base64 pbjson convention.
impl Debug for BlockHash<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a> From<&'a astria_core::sequencerblock::v1::block::Hash> for BlockHash<'a> {
    fn from(block_hash: &'a astria_core::sequencerblock::v1::block::Hash) -> Self {
        BlockHash(Cow::Borrowed(block_hash.as_bytes()))
    }
}

impl<'a> From<BlockHash<'a>> for astria_core::sequencerblock::v1::block::Hash {
    fn from(block_hash: BlockHash<'a>) -> Self {
        Self::new(block_hash.0.into_owned())
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
