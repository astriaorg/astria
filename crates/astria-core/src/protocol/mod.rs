use std::sync::Arc;

use bytes::Bytes;
use indexmap::IndexMap;
use transaction::v1::Transaction;

use crate::primitive::v1::RollupId;

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
pub fn group_rollup_data_submissions_in_signed_transaction_transactions_by_rollup_id(
    transactions: &[Arc<Transaction>],
) -> IndexMap<RollupId, Vec<Bytes>> {
    use prost::Message as _;

    use crate::sequencerblock::v1::block::RollupData;

    let mut map = IndexMap::new();
    for action in transactions.iter().flat_map(|tx| tx.actions()) {
        if let Some(action) = action.as_rollup_data_submission() {
            let txs_for_rollup: &mut Vec<Bytes> = map.entry(action.rollup_id).or_insert(vec![]);
            let rollup_data = RollupData::SequencedData(action.data.clone());
            txs_for_rollup.push(rollup_data.into_raw().encode_to_vec().into());
        }
    }
    map.sort_unstable_keys();
    map
}
