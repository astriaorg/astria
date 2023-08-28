use std::collections::BTreeMap;

use crate::MerkleTree;

fn concat_chain_id_and_merkle_root(chain_id: [u8; 32], root: [u8; 32]) -> [u8; 64] {
    let mut cated = [0u8; 64];
    cated[..32].copy_from_slice(&chain_id);
    cated[32..].copy_from_slice(&root);
    cated
}

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// The `BTreeMap` key is the chain ID and value is the transactions.
#[must_use]
pub fn generate_action_tree_leaves(
    chain_id_to_txs: BTreeMap<[u8; 32], Vec<Vec<u8>>>,
) -> Vec<[u8; 64]> {
    let mut leaves = Vec::with_capacity(chain_id_to_txs.len());
    for (chain_id, txs) in chain_id_to_txs {
        let root = MerkleTree::from_leaves(txs).root();
        let leaf = concat_chain_id_and_merkle_root(chain_id, root);
        leaves.push(leaf);
    }
    leaves
}
