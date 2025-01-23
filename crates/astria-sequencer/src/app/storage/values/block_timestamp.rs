use astria_eyre::eyre::bail;
use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug)]
pub(in crate::app) struct BlockTimestamp(tendermint::time::Time);

impl From<tendermint::time::Time> for BlockTimestamp {
    fn from(block_timestamp: tendermint::time::Time) -> Self {
        BlockTimestamp(block_timestamp)
    }
}

impl From<BlockTimestamp> for tendermint::time::Time {
    fn from(block_timestamp: BlockTimestamp) -> Self {
        block_timestamp.0
    }
}

impl BorshSerialize for BlockTimestamp {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Convert from `i128` to `u128` as the Go implementation of Borsh don't handle `i128`s.
        u128::try_from(self.0.unix_timestamp_nanos())
            .map_err(std::io::Error::other)?
            .serialize(writer)
    }
}

impl BorshDeserialize for BlockTimestamp {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let nanos = u128::deserialize_reader(reader)?;
        let timestamp = tendermint::time::Time::from_unix_timestamp(
            i64::try_from(nanos / 1_000_000_000).unwrap(),
            (nanos % 1_000_000_000) as u32,
        )
        .map_err(std::io::Error::other)?;
        Ok(BlockTimestamp(timestamp))
    }
}

impl From<BlockTimestamp> for crate::storage::StoredValue<'_> {
    fn from(block_timestamp: BlockTimestamp) -> Self {
        crate::storage::StoredValue::App(Value(ValueImpl::BlockTimestamp(block_timestamp)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BlockTimestamp {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::App(Value(ValueImpl::BlockTimestamp(block_timestamp))) =
            value
        else {
            bail!("app stored value type mismatch: expected block timestamp, found {value:?}");
        };
        Ok(block_timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_round_trip() {
        let timestamp = BlockTimestamp(tendermint::time::Time::now());
        let serialized = borsh::to_vec(&timestamp).unwrap();
        let deserialized: BlockTimestamp = borsh::from_slice(&serialized).unwrap();
        assert_eq!(timestamp.0, deserialized.0);
    }
}
