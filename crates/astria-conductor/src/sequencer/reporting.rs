use astria_core::sequencer::v1alpha1::{
    block::{
        FilteredSequencerBlock,
        RollupTransactions,
    },
    RollupId,
};
use indexmap::IndexMap;
use serde::ser::{
    Serialize,
    SerializeMap as _,
    SerializeStruct as _,
};

pub(super) struct ReportFilteredSequencerBlock<'a>(pub(super) &'a FilteredSequencerBlock);
impl<'a> Serialize for ReportFilteredSequencerBlock<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FilteredSequencerBlockInfo", 2)?;
        state.serialize_field("sequencer_height", &self.0.height().value())?;
        state.serialize_field("rollups", &ReportRollups(self.0.rollup_transactions()))?;
        state.end()
    }
}

struct ReportRollups<'a>(&'a IndexMap<RollupId, RollupTransactions>);

impl<'a> Serialize for ReportRollups<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, txes) in self.0 {
            map.serialize_entry(id, &txes.transactions().len())?;
        }
        map.end()
    }
}
