// @generated
/// The set of information which deterministic driver of block production
/// mustknow about a given rollup Block
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Block {
    /// The block number
    #[prost(uint32, tag="1")]
    pub number: u32,
    /// The hash of the block
    #[prost(bytes="vec", tag="2")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    /// The hash from the parent block
    #[prost(bytes="vec", tag="3")]
    pub parent_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// Timestamp on the block, standardized to google protobuf standard.
    #[prost(message, optional, tag="4")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
}
/// Fields which are indexed for finding blocks on a blockchain.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockIdentifier {
    #[prost(oneof="block_identifier::Identifier", tags="1, 2")]
    pub identifier: ::core::option::Option<block_identifier::Identifier>,
}
/// Nested message and enum types in `BlockIdentifier`.
pub mod block_identifier {
    #[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Identifier {
        #[prost(uint32, tag="1")]
        BlockNumber(u32),
        #[prost(bytes, tag="2")]
        BlockHash(::prost::alloc::vec::Vec<u8>),
    }
}
/// Used in GetBlock to find a single block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockRequest {
    #[prost(message, optional, tag="1")]
    pub identifier: ::core::option::Option<BlockIdentifier>,
}
/// Used in BatchGetBlocks, will find all or none based on the list of
/// identifiers.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksRequest {
    #[prost(message, repeated, tag="1")]
    pub identifiers: ::prost::alloc::vec::Vec<BlockIdentifier>,
}
/// The list of blocks in response to BatchGetBlocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksResponse {
    #[prost(message, repeated, tag="1")]
    pub blocks: ::prost::alloc::vec::Vec<Block>,
}
/// ExecuteBlockRequest contains all the information needed to create a new rollup
/// block.
///
/// This information comes from previous rollup blocks, as well as from sequencer
/// blocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteBlockRequest {
    /// The hash of previous block, which new block will be created on top of.
    #[prost(bytes="vec", tag="1")]
    pub prev_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// List of transactions to include in the new block.
    #[prost(bytes="vec", repeated, tag="2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// Timestamp to be used for new block.
    #[prost(message, optional, tag="3")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
}
/// The CommitmentState holds the block at each stage of sequencer commitment
/// level
///
/// A Valid CommitmentState:
/// - Block numbers are such that soft >= firm.
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the had of the canonical chain the firm block
///    must belong to.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitmentState {
    /// Soft commitment is the rollup block matching latest sequencer block.
    #[prost(message, optional, tag="1")]
    pub soft: ::core::option::Option<Block>,
    /// Firm commitment is achieved when data has been seen in DA.
    #[prost(message, optional, tag="2")]
    pub firm: ::core::option::Option<Block>,
}
/// There is only one CommitmentState object, so the request is empty.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommitmentStateRequest {
}
/// The CommitmentState to set, must include complete state.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateCommitmentStateRequest {
    #[prost(message, optional, tag="1")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
include!("astria.execution.v1alpha2.tonic.rs");
// @@protoc_insertion_point(module)