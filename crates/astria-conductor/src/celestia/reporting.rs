//! Various newtype-wrappers to emit serde-serialized tracing event fields.
use serde::ser::{
    Serialize,
    SerializeSeq,
    SerializeStruct,
};

use super::{
    ReconstructedBlock,
    ReconstructedBlocks,
};

pub(super) struct ReportReconstructedBlocks<'a>(pub(super) &'a ReconstructedBlocks);
impl Serialize for ReportReconstructedBlocks<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        const FIELDS: [&str; 2] = ["celestia_height", "reconstructed_blocks"];
        let mut state = serializer.serialize_struct("ReconstructedBlocksInfo", FIELDS.len())?;
        state.serialize_field(FIELDS[0], &self.0.celestia_height)?;
        state.serialize_field(FIELDS[1], &ReportReconstructedBlocksSeq(&self.0.blocks))?;
        state.end()
    }
}

struct ReportReconstructedBlocksSeq<'a>(&'a [ReconstructedBlock]);
impl Serialize for ReportReconstructedBlocksSeq<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for block in self.0 {
            seq.serialize_element(&ReportReconstructedBlock(block))?;
        }
        seq.end()
    }
}

struct ReportReconstructedBlock<'a>(&'a ReconstructedBlock);
impl Serialize for ReportReconstructedBlock<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        const FIELDS: [&str; 4] = [
            "celestia_height",
            "block_hash",
            "number_of_transactions",
            "from_celestia_height",
        ];
        let mut state = serializer.serialize_struct("ReconstructedBlockInfo", FIELDS.len())?;
        state.serialize_field(FIELDS[0], &self.0.celestia_height)?;
        state.serialize_field(FIELDS[1], &SerializeDisplay(&self.0.block_hash))?;
        state.serialize_field(FIELDS[2], &self.0.transactions.len())?;
        state.serialize_field(FIELDS[3], &self.0.celestia_height)?;
        state.end()
    }
}

struct SerializeDisplay<'a, T>(&'a T);

impl<T> Serialize for SerializeDisplay<'_, T>
where
    T: std::fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self.0)
    }
}
