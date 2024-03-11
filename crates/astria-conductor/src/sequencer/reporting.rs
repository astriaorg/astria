use astria_core::sequencer::v1alpha1::{
    block::{
        RollupTransactions,
        SequencerBlock,
    },
    RollupId,
};
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
pub(super) struct ReportBlock<'a>(
    #[serde(serialize_with = "serialize_report_block")] pub(super) &'a SequencerBlock,
);

#[derive(serde::Serialize)]
#[serde(transparent)]
struct ReportRollups<'a>(
    #[serde(serialize_with = "serialize_report_rollups")]
    &'a IndexMap<RollupId, RollupTransactions>,
);

fn serialize_report_block<S>(block: &SequencerBlock, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let mut state = serializer.serialize_struct("SequencerBlock", 2)?;
    state.serialize_field("height", &block.height().value())?;
    state.serialize_field("rollups", &ReportRollups(block.rollup_transactions()))?;
    state.end()
}

fn serialize_report_rollups<S>(
    rollups: &IndexMap<RollupId, RollupTransactions>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::{
        SerializeMap,
        // Serializer,
    };
    let mut map = serializer.serialize_map(Some(rollups.len()))?;
    for (id, txes) in rollups {
        map.serialize_entry(id, &txes.transactions().len())?;
    }
    map.end()
}
