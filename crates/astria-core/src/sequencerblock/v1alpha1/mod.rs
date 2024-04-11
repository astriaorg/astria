pub mod block;
pub mod celestia;

pub use block::{
    RollupTransactions,
    SequencerBlock,
};
pub use celestia::{
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
};
use indexmap::IndexMap;
use sha2::{
    Digest as _,
    Sha256,
};

use crate::{
    generated::sequencerblock::v1alpha1 as raw,
    sequencer::v1::{
        derive_merkle_tree_from_rollup_txs,
        RollupId,
    },
};

pub(crate) fn are_rollup_ids_included<'a, TRollupIds: 'a>(
    ids: TRollupIds,
    proof: &merkle::Proof,
    data_hash: [u8; 32],
) -> bool
where
    TRollupIds: IntoIterator<Item = RollupId>,
{
    let tree = merkle::Tree::from_leaves(ids);
    let hash_of_root = Sha256::digest(tree.root());
    proof.verify(&hash_of_root, data_hash)
}

pub(crate) fn are_rollup_txs_included(
    rollup_datas: &IndexMap<RollupId, RollupTransactions>,
    rollup_proof: &merkle::Proof,
    data_hash: [u8; 32],
) -> bool {
    let rollup_datas = rollup_datas
        .iter()
        .map(|(rollup_id, tx_data)| (rollup_id, tx_data.transactions()));
    let rollup_tree = derive_merkle_tree_from_rollup_txs(rollup_datas);
    let hash_of_rollup_root = Sha256::digest(rollup_tree.root());
    rollup_proof.verify(&hash_of_rollup_root, data_hash)
}

fn do_rollup_transaction_match_root(
    rollup_transactions: &RollupTransactions,
    root: [u8; 32],
) -> bool {
    let id = rollup_transactions.id();
    rollup_transactions
        .proof()
        .audit()
        .with_root(root)
        .with_leaf_builder()
        .write(id.as_ref())
        .write(&merkle::Tree::from_leaves(rollup_transactions.transactions()).root())
        .finish_leaf()
        .perform()
}
