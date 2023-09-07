use std::collections::HashMap;

use astria_sequencer_validation::MerkleTree;
use tendermint::{
    block::Header,
    Hash,
};

use crate::{
    RawSequencerBlockData,
    SequencerBlockData,
};

#[allow(clippy::missing_panics_doc)]
#[must_use]
/// Returns a default tendermint block header for test purposes.
pub fn default_header() -> Header {
    use tendermint::{
        account,
        block::{
            header::Version,
            Height,
        },
        chain,
        hash::AppHash,
        Time,
    };

    Header {
        version: Version {
            block: 0,
            app: 0,
        },
        chain_id: chain::Id::try_from("test").unwrap(),
        height: Height::from(1u32),
        time: Time::now(),
        last_block_id: None,
        last_commit_hash: None,
        data_hash: None,
        validators_hash: Hash::Sha256([0; 32]),
        next_validators_hash: Hash::Sha256([0; 32]),
        consensus_hash: Hash::Sha256([0; 32]),
        app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        last_results_hash: None,
        evidence_hash: None,
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    }
}

/// Creates a [`RawSequencerBlockData`] that successfully converts to
/// [`SequencerBlockData`]. Salt makes sure every block has a unique hash.
pub fn new_raw_block(salt: u8) -> RawSequencerBlockData {
    let action_tree_root = [9u8; 32];

    let transactions = vec![
        action_tree_root.to_vec(),
        vec![salt, 0x22, 0x33],
        vec![0x44, 0x55, 0x66],
        vec![0x77, 0x88, 0x99],
    ];
    let tree = MerkleTree::from_leaves(transactions);
    let action_tree_root_inclusion_proof = tree.prove_inclusion(0).unwrap();

    let mut header = default_header();
    header.data_hash = Some(Hash::try_from(tree.root().to_vec()).unwrap());

    let block_hash = header.hash();

    RawSequencerBlockData {
        block_hash,
        header,
        last_commit: None,
        rollup_data: HashMap::new(),
        action_tree_root,
        action_tree_root_inclusion_proof,
    }
}

/// Allows mutation to specific fields of raw block, e.g. height, and then conversion to
/// [`SequencerBlockData`].
pub fn from_raw(mut raw_block: RawSequencerBlockData) -> SequencerBlockData {
    raw_block.block_hash = raw_block.header.hash(); // recompute hash

    SequencerBlockData::try_from_raw(raw_block).unwrap()
}
