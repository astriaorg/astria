use std::collections::BTreeMap;

use ct_merkle::CtMerkleTree;

use crate::MerkleTree;

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// The `BTreeMap` key is the chain ID and value is the transactions.
#[must_use]
pub fn generate_action_tree_leaves<T: AsRef<[u8]>>(
    chain_id_to_txs: BTreeMap<T, Vec<Vec<u8>>>,
) -> Vec<Vec<u8>> {
    let mut leaves = Vec::new();
    for (chain_id, txs) in chain_id_to_txs {
        let root = MerkleTree::from_leaves(txs).root();
        let mut leaf = chain_id.as_ref().to_vec();
        leaf.append(&mut root.to_vec());
        leaves.push(leaf);
    }
    leaves
}

pub fn generate_commitment<'a, T: IntoIterator<Item = &'a [u8]> + 'a>(input: T) -> [u8; 32] {
    let mut tree = CtMerkleTree::new();
    for elem in input {
        tree.push(elem.to_vec());
    }
    MerkleTree::from_inner_tree(tree).root()
}
