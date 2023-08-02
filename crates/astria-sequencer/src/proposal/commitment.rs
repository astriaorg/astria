use std::collections::BTreeMap;

use anyhow::{
    Context as _,
    Error,
    Result,
};
use bytes::Bytes;
use tendermint::merkle::simple_hash_from_byte_vectors;

use crate::transaction::Signed;

/// Called when we receive a `PrepareProposal` or `ProcessProposal` consensus message.
///
/// In the case of `PrepareProposal`, we use this function to generate the `commitment_tx`
/// to be placed at the start of the block.
///
/// In the case of `ProcessProposal`, we use this function to generate and verify the
/// `commitment_tx` expected at the start of the block.
///
/// This function sorts the block's `secondary::Action`s contained within the transactions
/// using their `chain_id`. It then returns the merkle root of the tree where each leaf is
/// a commitment of `secondary::Action`s with the same `chain_id`. The leaves are ordered
/// by namespace in ascending order, where `namespace(chain_id) = Sha256(chain_id)[0..10]`.
/// This structure can be referred to as the "action tree".
///
/// The leaf, which contains a commitment to every action with the same `chain_id`, is currently
/// implemented as ( `namespace(chain_id)` || root of merkle tree of the `secondary::Action`s ).
/// This is somewhat arbitrary, but could be useful for proof of an action within the action tree.
pub(crate) fn generate_transaction_commitment(txs_bytes: &[Bytes]) -> Result<[u8; 32]> {
    let txs = txs_bytes
        .iter()
        .map(|tx_bytes| Signed::try_from_slice(tx_bytes))
        .collect::<Result<Vec<Signed>, Error>>()
        .context("failed to deserialize transactions")?;

    let chain_id_to_txs = sort_txs_by_chain_id(&txs);

    let mut leaves: Vec<Vec<u8>> = vec![];
    for (chain_id, txs) in chain_id_to_txs {
        let chain_id_root =
            simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(&txs);
        let mut leaf = get_namespace(&chain_id).to_vec();
        leaf.append(&mut chain_id_root.to_vec());
        leaves.push(leaf);
    }

    Ok(simple_hash_from_byte_vectors::<
        tendermint::crypto::default::Sha256,
    >(&leaves))
}

/// returns an 10-byte namespace given a byte slice.
/// TODO: duplicate in `astria-sequencer-relayer/src/types.rs`
fn get_namespace(bytes: &[u8]) -> [u8; 10] {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result[0..10].to_owned().try_into().unwrap()
}

/// Sorts the actions within the transactions by their `chain_id`.
/// Within an entry, actions are ordered by their transaction index within a block.
fn sort_txs_by_chain_id(txs: &[Signed]) -> BTreeMap<Vec<u8>, Vec<Vec<u8>>> {
    let mut rollup_txs = BTreeMap::new();

    for tx in txs.iter() {
        tx.transaction().actions().iter().for_each(|action| {
            if let Some(action) = action.as_sequence() {
                let txs = rollup_txs
                    .entry(action.chain_id().to_vec())
                    .or_insert(vec![]);
                txs.push(action.data().to_vec());
            }
        });
    }

    rollup_txs
}
