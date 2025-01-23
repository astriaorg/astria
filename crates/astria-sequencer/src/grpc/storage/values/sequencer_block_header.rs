use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::{
    primitive::v1::ADDRESS_LEN,
    sequencerblock::v1::block::{
        SequencerBlockHeader as DomainSequencerBlockHeader,
        SequencerBlockHeaderParts,
    },
};
use astria_eyre::eyre::bail;
use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug)]
struct ChainId<'a>(Cow<'a, tendermint::chain::Id>);

impl<'a> From<&'a tendermint::chain::Id> for ChainId<'a> {
    fn from(chain_id: &'a tendermint::chain::Id) -> Self {
        ChainId(Cow::Borrowed(chain_id))
    }
}

impl<'a> From<ChainId<'a>> for tendermint::chain::Id {
    fn from(chain_id: ChainId<'a>) -> Self {
        chain_id.0.into_owned()
    }
}

impl BorshSerialize for ChainId<'_> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.as_str().serialize(writer)
    }
}

impl BorshDeserialize for ChainId<'_> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let chain_id_str = String::deserialize_reader(reader)?;
        let chain_id =
            tendermint::chain::Id::try_from(chain_id_str).map_err(std::io::Error::other)?;
        Ok(ChainId(Cow::Owned(chain_id)))
    }
}

#[derive(Debug)]
struct BlockTimestamp(tendermint::time::Time);

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

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct SequencerBlockHeader<'a> {
    chain_id: ChainId<'a>,
    height: u64,
    time: BlockTimestamp,
    rollup_transactions_root: Cow<'a, [u8; 32]>,
    data_hash: Cow<'a, [u8; 32]>,
    proposer_address: [u8; ADDRESS_LEN],
}

impl Debug for SequencerBlockHeader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SequencerBlockHeader")
            .field("chain_id", &self.chain_id)
            .field("height", &self.height)
            .field("time", &self.time)
            .field(
                "rollup_transactions_root",
                &base64(self.rollup_transactions_root.as_slice()).to_string(),
            )
            .field("data_hash", &base64(self.data_hash.as_slice()).to_string())
            .field(
                "proposer_address",
                &base64(self.proposer_address.as_slice()).to_string(),
            )
            .finish()
    }
}

impl<'a> From<&'a DomainSequencerBlockHeader> for SequencerBlockHeader<'a> {
    fn from(header: &'a DomainSequencerBlockHeader) -> Self {
        const _: () = assert!(ADDRESS_LEN == tendermint::account::LENGTH);
        let mut proposer_address = [0; ADDRESS_LEN];
        proposer_address.copy_from_slice(header.proposer_address().as_bytes());

        SequencerBlockHeader {
            chain_id: header.chain_id().into(),
            height: header.height().value(),
            time: header.time().into(),
            rollup_transactions_root: Cow::Borrowed(header.rollup_transactions_root()),
            data_hash: Cow::Borrowed(header.data_hash()),
            proposer_address,
        }
    }
}

impl<'a> From<SequencerBlockHeader<'a>> for DomainSequencerBlockHeader {
    fn from(header: SequencerBlockHeader<'a>) -> Self {
        let height = tendermint::block::Height::try_from(header.height)
            .expect("should not be able to store invalid height");
        DomainSequencerBlockHeader::unchecked_from_parts(SequencerBlockHeaderParts {
            chain_id: header.chain_id.into(),
            height,
            time: header.time.into(),
            rollup_transactions_root: header.rollup_transactions_root.into_owned(),
            data_hash: header.data_hash.into_owned(),
            proposer_address: tendermint::account::Id::new(header.proposer_address),
        })
    }
}

impl<'a> From<SequencerBlockHeader<'a>> for crate::storage::StoredValue<'a> {
    fn from(block_header: SequencerBlockHeader<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::SequencerBlockHeader(block_header)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for SequencerBlockHeader<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::SequencerBlockHeader(block_header))) =
            value
        else {
            bail!(
                "grpc stored value type mismatch: expected sequencer block header, found {value:?}"
            );
        };
        Ok(block_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_serialization_round_trip() {
        let id: tendermint::chain::Id = "a".parse().unwrap();
        let chain_id = ChainId::from(&id);
        let serialized = borsh::to_vec(&chain_id).unwrap();
        let deserialized: ChainId = borsh::from_slice(&serialized).unwrap();
        assert_eq!(chain_id.0, deserialized.0);
    }

    #[test]
    fn block_timestamp_serialization_round_trip() {
        let timestamp = BlockTimestamp(tendermint::time::Time::now());
        let serialized = borsh::to_vec(&timestamp).unwrap();
        let deserialized: BlockTimestamp = borsh::from_slice(&serialized).unwrap();
        assert_eq!(timestamp.0, deserialized.0);
    }
}
