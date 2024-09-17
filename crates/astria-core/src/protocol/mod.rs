use bytes::Bytes;
use indexmap::IndexMap;
use transaction::v1alpha1::{
    action_groups::{
        ActionGroup,
        BundlableGeneralAction,
    },
    SignedTransaction,
};

use crate::primitive::v1::RollupId;

pub mod abci;
pub mod account;
pub mod asset;
pub mod bridge;
pub mod genesis;
pub mod memos;
pub mod transaction;

#[cfg(any(feature = "test-utils", test))]
pub mod test_utils;

/// Extracts all data within [`SequenceAction`]s in the given [`SignedTransaction`]s, wraps them as
/// [`RollupData::SequencedData`] and groups them by [`RollupId`].
///
/// TODO: This can all be done in-place once <https://github.com/rust-lang/rust/issues/80552> is stabilized.
pub fn group_sequence_actions_in_signed_transaction_transactions_by_rollup_id(
    signed_transactions: &[SignedTransaction],
) -> IndexMap<RollupId, Vec<Bytes>> {
    use prost::Message as _;

    use crate::sequencerblock::v1alpha1::block::RollupData;

    let mut map = IndexMap::new();
    for tx in signed_transactions {
        if let ActionGroup::BundlableGeneral(general_bundle) = tx.actions() {
            for action in &general_bundle.actions {
                if let BundlableGeneralAction::Sequence(sequence_action) = action {
                    let txs_for_rollup: &mut Vec<Bytes> =
                        map.entry(sequence_action.rollup_id).or_insert(vec![]);
                    let rollup_data = RollupData::SequencedData(sequence_action.data.clone());
                    txs_for_rollup.push(rollup_data.into_raw().encode_to_vec().into());
                }
            }
        }
    }
    map.sort_unstable_keys();
    map
}
