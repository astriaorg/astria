use indexmap::IndexMap;
use transaction::v1alpha1::SignedTransaction;

use crate::primitive::v1::RollupId;

pub mod abci;
pub mod account;
pub mod transaction;

#[cfg(any(feature = "test-utils", test))]
pub mod test_utils;

/// Extracts all data within [`SequenceAction`]s in the given [`SignedTransaction`]s, wraps them as
/// [`RollupData::SequencedData`] and groups them by [`RollupId`].
///
/// TODO: This can all be done in-place once <https://github.com/rust-lang/rust/issues/80552> is stabilized.
pub fn group_sequence_actions_in_signed_transaction_transactions_by_rollup_id(
    signed_transactions: &[SignedTransaction],
) -> IndexMap<RollupId, Vec<Vec<u8>>> {
    use prost::Message as _;

    use crate::sequencerblock::v1alpha1::block::RollupData;

    let mut map = IndexMap::new();
    for action in signed_transactions
        .iter()
        .flat_map(SignedTransaction::actions)
    {
        if let Some(action) = action.as_sequence() {
            let txs_for_rollup: &mut Vec<Vec<u8>> = map.entry(action.rollup_id).or_insert(vec![]);
            let rollup_data = RollupData::SequencedData(action.data.clone());
            txs_for_rollup.push(rollup_data.into_raw().encode_to_vec());
        }
    }
    map.sort_unstable_keys();
    map
}
