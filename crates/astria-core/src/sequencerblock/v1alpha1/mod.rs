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
    sequencerblock::Protobuf,
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
    let id = rollup_transactions.rollup_id();
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

impl Protobuf for merkle::Proof {
    type Error = merkle::audit::InvalidProof;
    type Raw = raw::Proof;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        // XXX: Implementing this by cloning is ok because `audit_path`
        //      has to be cloned always due to `UncheckedProof`'s constructor.
        Self::try_from_raw(raw.clone())
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            audit_path,
            leaf_index,
            tree_size,
        } = raw;
        let leaf_index = leaf_index.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        let tree_size = tree_size.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        Self::unchecked()
            .audit_path(audit_path)
            .leaf_index(leaf_index)
            .tree_size(tree_size)
            .try_into_proof()
    }

    fn to_raw(&self) -> Self::Raw {
        // XXX: Implementing in terms of clone is ok because the fields would need to be cloned
        // anyway.
        self.clone().into_raw()
    }

    fn into_raw(self) -> Self::Raw {
        let merkle::audit::UncheckedProof {
            audit_path,
            leaf_index,
            tree_size,
        } = self.into_unchecked();
        Self::Raw {
            audit_path,
            leaf_index: leaf_index.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
            tree_size: tree_size.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
        }
    }
}
