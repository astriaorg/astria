use std::collections::BTreeMap;

use crate::MerkleTree;

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// The `BTreeMap` key is the chain ID and value is the transactions.
#[must_use]
pub fn generate_action_tree_leaves(
    chain_id_to_txs: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
) -> Vec<Vec<u8>> {
    let mut leaves: Vec<Vec<u8>> = vec![];
    for (chain_id, txs) in chain_id_to_txs {
        let chain_id_root = MerkleTree::from_leaves(txs).root();
        let mut leaf = chain_id.clone();
        leaf.append(&mut chain_id_root.to_vec());
        leaves.push(leaf);
    }
    leaves
}
