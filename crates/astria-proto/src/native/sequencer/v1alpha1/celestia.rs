use sequencer_valiation::InclusionProof;

use crate::generated::sequencer::v1alpha1 as raw;

#[derive(Clone, PartialEq)]
pub struct CelestiaRollupData {
    /// The hash of the sequencer block. Must be 32 bytes.
    pub sequencer_block_hash: [u8; 32],
    /// The human readable chain ID identifiying the rollup these transactions belong to.
    pub chain_id: String,
    /// A list of opaque bytes that are serialized rollup transactions.
    pub transactions: Vec<Vec<u8>>,
    /// The proof that the action tree root was included in
    /// `sequencer_block_header.data_hash` (to be found in `CelestiaSequencerData`).
    /// The inclusion of these transactions in the original sequencer block
    /// can be verified using the action tree root stored in `CelestiaHeader`.
    pub action_tree_inclusion_proof: InclusionProof,
}

pub struct CelestiaSequencerData {
    /// The hash of the sequencer block. Must be 32 bytes.
    pub sequencer_block_hash: [u8; 32],
    /// The original cometbft header that was the input to this sequencer block.
    pub sequencer_block_header: tendermint_proto::types::Header,
    /// The commit/set of signatures that commited this block.
    pub sequencer_block_last_commit: tendermint_proto::types::Commit,
    /// The namespaces under which rollup transactions belonging to the sequencer
    /// block identified by `sequencer_block_hash` where submitted to celestia.
    /// The bytes must convert to a celestia v0 namespace.
    pub rollup_namespaces: Vec<[u8; 29]>,
    /// The root of the action tree of this block. Must be 32 bytes.
    pub action_tree_root: [u8; 32],
    /// The proof that the action tree root was included in `header.data_hash`.
    pub action_tree_inclusion_proof: InclusionProof,
}
