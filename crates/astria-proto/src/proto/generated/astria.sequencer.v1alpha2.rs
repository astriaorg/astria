// @generated
/// A merkle proof of inclusion of a leaf at `index` in a tree
/// of size `num_leaves`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InclusionProof {
    /// leaf index of value to be proven
    #[prost(uint64, tag="1")]
    pub index: u64,
    /// total number of leaves in the tree
    #[prost(uint64, tag="2")]
    pub num_leaves: u64,
    /// the merkle proof itself. This proof is derived from a RFC 6962 compliant Merkle tree.
    #[prost(bytes="vec", tag="3")]
    pub inclusion_proof: ::prost::alloc::vec::Vec<u8>,
}
/// `RollupTransactions` are a sequence of opaque bytes together with the human
/// readable `chain_id` of the rollup they belong to.
///
/// The binary encoding is understood as an implementation detail of the
/// services sending and receiving the transactions.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupTransactions {
    /// The human readable name of the rollup these transactions belong to.
    #[prost(string, tag="1")]
    pub chain_id: ::prost::alloc::string::String,
    /// The serialized opaque bytes of the rollup transactions.
    #[prost(bytes="vec", repeated, tag="2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
/// `SequencerBlock` is constructed from a tendermint/cometbft block by
/// converting its opaque `data` bytes into sequencer specific types.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlock {
    /// The hash of the sequencer block. Must be 32 bytes.
    #[prost(bytes="vec", tag="1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    /// The original cometbft header that was the input to this sequencer block.
    #[prost(message, optional, tag="2")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The commit/set of signatures that commited this block.
    #[prost(message, optional, tag="3")]
    pub last_commit: ::core::option::Option<::tendermint_proto::types::Commit>,
    /// The collection of rollup transactions that were included in this block.
    #[prost(message, optional, tag="4")]
    pub rollup_transactions: ::core::option::Option<RollupTransactions>,
    /// The root of the action tree of this block. Must be 32 bytes.
    #[prost(bytes="vec", tag="5")]
    pub action_tree_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the action tree root was included in `header.data_hash`.
    #[prost(message, optional, tag="6")]
    pub action_tree_inclusion_proof: ::core::option::Option<InclusionProof>,
}
/// A collection of transactions belonging to a specific rollup that are submitted to celestia.
///
/// The transactions contained in the item belong to a rollup identified
/// by `chain_id`, and were included in the sequencer block identified
/// by `sequencer_block_hash`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CelestiaRollupData {
    /// The hash of the sequencer block. Must be 32 bytes.
    #[prost(bytes="vec", tag="1")]
    pub sequencer_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// The human readable chain ID identifiying the rollup these transactions belong to.
    #[prost(string, tag="2")]
    pub chain_id: ::prost::alloc::string::String,
    /// A list of opaque bytes that are serialized rollup transactions.
    #[prost(bytes="vec", repeated, tag="3")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that the action tree root was included in
    /// `sequencer_block_header.data_hash` (to be found in `CelestiaHeader`).
    /// The inclusion of these transactions in the original sequencer block
    /// can be verified using the action tree root stored in `CelestiaHeader`.
    #[prost(message, optional, tag="4")]
    pub action_tree_inclusion_proof: ::core::option::Option<InclusionProof>,
}
/// The metadata of a sequencer block that is submitted to celestia.
///
/// It is created by splitting up a `SequencerBlockData` into a "header"
/// (this `CelestiaSequencerData`), and a list of `CelestiaRollupData` items.
///
/// The original sequencer block is identified by its `block_hash`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CelestiaSequencerData {
    /// The hash of the sequencer block. Must be 32 bytes.
    #[prost(bytes="vec", tag="1")]
    pub sequencer_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// The original cometbft header that was the input to this sequencer block.
    #[prost(message, optional, tag="2")]
    pub sequencer_block_header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The commit/set of signatures that commited this block.
    #[prost(message, optional, tag="3")]
    pub sequencer_bock_last_commit: ::core::option::Option<::tendermint_proto::types::Commit>,
    /// The namespaces under which rollup transactions belonging to the sequencer
    /// block identified by `sequencer_block_hash` where submitted to celestia.
    /// The bytes must convert to a celestia v0 namespace.
    #[prost(bytes="vec", repeated, tag="4")]
    pub rollup_namespaces: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The root of the action tree of this block. Must be 32 bytes.
    #[prost(bytes="vec", tag="5")]
    pub action_tree_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the action tree root was included in `header.data_hash`.
    #[prost(message, optional, tag="6")]
    pub action_tree_inclusion_proof: ::core::option::Option<InclusionProof>,
}
// @@protoc_insertion_point(module)
