// @generated
/// A response containing the balance of an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceResponse {
    #[prost(uint64, tag="2")]
    pub height: u64,
    #[prost(message, optional, tag="3")]
    pub balance: ::core::option::Option<super::super::primitive::v1::Uint128>,
}
/// A response containing the current nonce for an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NonceResponse {
    #[prost(uint64, tag="2")]
    pub height: u64,
    #[prost(uint32, tag="3")]
    pub nonce: u32,
}
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
    /// must be a multiple of 32 bytes.
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
    /// Opaque bytes identifying the rollup that these transactions belong to. Must be 32 bytes.
    #[prost(bytes="vec", tag="1")]
    pub chain_id: ::prost::alloc::vec::Vec<u8>,
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
    #[prost(message, repeated, tag="4")]
    pub rollup_transactions: ::prost::alloc::vec::Vec<RollupTransactions>,
    /// The root of the action tree of this block. Must be 32 bytes.
    #[prost(bytes="vec", tag="5")]
    pub action_tree_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the action tree root was included in `header.data_hash`.
    #[prost(message, optional, tag="6")]
    pub action_tree_inclusion_proof: ::core::option::Option<InclusionProof>,
    /// The root of the merkle tree constructed form the chain IDs of the rollup
    /// transactions in this block. Must be 32 bytes.
    #[prost(bytes="vec", tag="7")]
    pub chain_ids_commitment: ::prost::alloc::vec::Vec<u8>,
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
    /// Opaque bytes identifying the rollup that these transactions belong to. Must be 32 bytes.
    #[prost(bytes="vec", tag="2")]
    pub chain_id: ::prost::alloc::vec::Vec<u8>,
    /// A list of opaque bytes that are serialized rollup transactions.
    #[prost(bytes="vec", repeated, tag="3")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that the action tree root was included in
    /// `sequencer_block_header.data_hash` (to be found in `CelestiaSequencerData`).
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
    pub sequencer_block_last_commit: ::core::option::Option<::tendermint_proto::types::Commit>,
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
/// `IndexedTransaction` represents a sequencer transaction along with the index
/// it was originally in the sequencer block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IndexedTransaction {
    /// TODO: this is usize - how to define for variable size?
    #[prost(uint64, tag="1")]
    pub block_index: u64,
    #[prost(bytes="vec", tag="2")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupNamespace {
    #[prost(uint64, tag="1")]
    pub block_height: u64,
    #[prost(bytes="vec", tag="2")]
    pub namespace: ::prost::alloc::vec::Vec<u8>,
}
/// `RollupNamespaceData`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupNamespaceData {
    #[prost(bytes="vec", tag="1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag="2")]
    pub rollup_txs: ::prost::alloc::vec::Vec<IndexedTransaction>,
}
/// `SequencerNamespaceData`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerNamespaceData {
    #[prost(bytes="vec", tag="1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    #[prost(message, repeated, tag="3")]
    pub sequencer_txs: ::prost::alloc::vec::Vec<IndexedTransaction>,
    #[prost(message, repeated, tag="4")]
    pub rollup_namespaces: ::prost::alloc::vec::Vec<RollupNamespace>,
}
/// `SignedNamespaceData?`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedNamespaceData {
    #[prost(bytes="vec", tag="1")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
/// `SignedTransaction` is a transaction that has
/// been signed by the given public key.
/// It wraps an `UnsignedTransaction` with a 
/// signature and public key.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedTransaction {
    #[prost(bytes="vec", tag="1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub transaction: ::core::option::Option<UnsignedTransaction>,
}
/// `UnsignedTransaction` is a transaction that does 
/// not have an attached signature.
/// Note: `value` must be set, it cannot be `None`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(uint32, tag="1")]
    pub nonce: u32,
    #[prost(message, repeated, tag="2")]
    pub actions: ::prost::alloc::vec::Vec<Action>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(oneof="action::Value", tags="1, 2")]
    pub value: ::core::option::Option<action::Value>,
}
/// Nested message and enum types in `Action`.
pub mod action {
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(message, tag="1")]
        TransferAction(super::TransferAction),
        #[prost(message, tag="2")]
        SequenceAction(super::SequenceAction),
    }
}
/// `TransferAction` represents a value transfer transaction.
///
/// Note: all values must be set (ie. not `None`), otherwise it will
/// be considered invalid by the sequencer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransferAction {
    #[prost(bytes="vec", tag="1")]
    pub to: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
}
/// `SequenceAction` represents a transaction destined for another
/// chain, ordered by the sequencer.
///
/// It contains the chain ID of the destination chain, and the
/// opaque transaction data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequenceAction {
    /// Opaque bytes identifying the rollup that this transaction belong to. Must be 32 bytes.
    #[prost(bytes="vec", tag="1")]
    pub chain_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
// @@protoc_insertion_point(module)
