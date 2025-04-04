use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::{
        FilteredSequencerBlock,
        RollupTransactions,
    },
};
use indexmap::IndexMap;
use serde::ser::{
    Serialize,
    SerializeMap as _,
    SerializeStruct as _,
};

pub(super) struct ReportFilteredSequencerBlock<'a>(pub(super) &'a FilteredSequencerBlock);
impl Serialize for ReportFilteredSequencerBlock<'_> {
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

impl Serialize for ReportRollups<'_> {
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

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::RollupId,
        protocol::test_utils::ConfigureSequencerBlock,
        sequencerblock::v1::block::FilteredSequencerBlock,
    };
    use insta::assert_json_snapshot;

    use crate::sequencer::reporting::{
        ReportFilteredSequencerBlock,
        ReportRollups,
    };

    const ROLLUP_42: RollupId = RollupId::new([42u8; 32]);
    const ROLLUP_69: RollupId = RollupId::new([69u8; 32]);

    fn snapshot_block() -> FilteredSequencerBlock {
        let block = ConfigureSequencerBlock {
            height: 100,
            sequence_data: vec![
                (ROLLUP_42, b"hello".to_vec()),
                (ROLLUP_42, b"hello world".to_vec()),
                (ROLLUP_69, b"hello world".to_vec()),
            ],
            ..Default::default()
        }
        .make();

        block.into_filtered_block([ROLLUP_42, ROLLUP_69])
    }

    #[test]
    fn snapshots() {
        let block = snapshot_block();

        assert_json_snapshot!(ReportRollups(block.rollup_transactions()));
        assert_json_snapshot!(ReportFilteredSequencerBlock(&block));
    }
}
