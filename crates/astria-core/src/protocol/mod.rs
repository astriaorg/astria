use bytes::Bytes;
use indexmap::IndexMap;
use prost::Message as _;

use crate::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::RollupData,
};

pub mod abci;
pub mod account;
pub mod asset;
pub mod bridge;
pub mod fees;
pub mod genesis;
pub mod memos;
pub mod price_feed;
pub mod transaction;

#[cfg(any(feature = "test-utils", test))]
pub mod test_utils;

/// Extracts all data within [`Sequence`] actions in the given [`Transaction`]s, wraps them as
/// [`RollupData::SequencedData`] and groups them by [`RollupId`].
///
/// TODO: This can all be done in-place once <https://github.com/rust-lang/rust/issues/80552> is stabilized.
pub fn group_rollup_data_submissions_by_rollup_id<'a, I>(
    rollup_data_bytes: I,
) -> IndexMap<RollupId, Vec<Bytes>>
where
    I: Iterator<Item = (&'a RollupId, &'a Bytes)>,
{
    let mut map = IndexMap::new();
    for (rollup_id, data_bytes) in rollup_data_bytes {
        let txs_for_rollup: &mut Vec<Bytes> = map.entry(*rollup_id).or_insert(vec![]);
        let rollup_data = RollupData::SequencedData(data_bytes.clone());
        txs_for_rollup.push(rollup_data.into_raw().encode_to_vec().into());
    }
    map.sort_unstable_keys();
    map
}
