use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug)]
pub(crate) struct BlockTimestamp(tendermint::time::Time);

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

impl<'a> TryFrom<StoredValue<'a>> for BlockTimestamp {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::BlockTimestamp(block_timestamp) = value else {
            return Err(super::type_mismatch("block timestamp", &value));
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
