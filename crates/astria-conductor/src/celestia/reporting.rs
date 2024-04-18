//! Various newtype-wrappers to emit serde-serialized tracing event fields.
use serde::ser::{
    Serialize,
    SerializeSeq,
    SerializeStruct,
};
use telemetry::display::base64;

use super::{
    ReconstructedBlock,
    ReconstructedBlocks,
};
use crate::block_cache::GetSequencerHeight;

pub(super) struct ReportSequencerHeights<'a, T>(pub(super) &'a [T]);

impl<'a, T> Serialize for ReportSequencerHeights<'a, T>
where
    T: GetSequencerHeight,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for elem in self.0 {
            seq.serialize_element(&elem.get_height())?;
        }
        seq.end()
    }
}

pub(super) struct ReportReconstructedBlocks<'a>(pub(super) &'a ReconstructedBlocks);
impl<'a> Serialize for ReportReconstructedBlocks<'a> {
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
impl<'a> Serialize for ReportReconstructedBlocksSeq<'a> {
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
impl<'a> Serialize for ReportReconstructedBlock<'a> {
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
        state.serialize_field(FIELDS[1], &base64(&self.0.block_hash))?;
        state.serialize_field(FIELDS[2], &self.0.transactions.len())?;
        state.serialize_field(FIELDS[3], &self.0.celestia_height)?;
        state.end()
    }
}
