// @generated
/// The set of information which deterministic driver may to know about a given executed Block
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Block {
    /// The block number
    #[prost(uint32, tag="1")]
    pub number: u32,
    /// The hash of the block
    #[prost(bytes="vec", tag="2")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    /// Timestamp on the block, standardized to google protobuf standard.
    #[prost(message, optional, tag="3")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
}
/// These fields should be indexed on most block chains, and can be used to identify a block.
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockRequest {
    #[prost(message, optional, tag="1")]
    pub identifier: ::core::option::Option<BlockIdentifier>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockResponse {
    #[prost(message, optional, tag="1")]
    pub block: ::core::option::Option<Block>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksRequest {
    #[prost(message, repeated, tag="1")]
    pub identifiers: ::prost::alloc::vec::Vec<BlockIdentifier>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksResponse {
    #[prost(message, repeated, tag="1")]
    pub blocks: ::prost::alloc::vec::Vec<Block>,
}
/// CreateBlockRequest contains all the information needed to create a new executed block.
///
/// This information comes from previous execution blocks, as well as from sequencer blocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateBlockRequest {
    /// The hash of the block which will be executed on top of
    #[prost(bytes="vec", tag="1")]
    pub prev_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// List of transactions to include in the new block
    #[prost(bytes="vec", repeated, tag="2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The timestamp which should be used for new block
    #[prost(message, optional, tag="3")]
    pub timestamp: ::core::option::Option<::prost_types::Timestamp>,
}
/// CreateBlockResponse is returned after calling CreateBlock
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateBlockResponse {
    /// The executed block which was created.
    #[prost(message, optional, tag="1")]
    pub block: ::core::option::Option<Block>,
}
/// The CommitmentState holds the block at each stage of sequencer commitment level
///
/// A Valid CommitmentState:
/// - Block numbers are such that soft+1 >= head >= soft >= firm.
/// - No blocks ever decrease in block number, only head may stay the same and have other changes
/// - The chain defined by head contains soft and firm blocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitmentState {
    /// The head is the top of the executed chain
    #[prost(message, optional, tag="1")]
    pub head: ::core::option::Option<Block>,
    /// Soft commitment is the executed block matching sequencer block with full consensus.
    #[prost(message, optional, tag="2")]
    pub soft: ::core::option::Option<Block>,
    /// Firm commitment 
    #[prost(message, optional, tag="3")]
    pub firm: ::core::option::Option<Block>,
}
/// There is only one CommitmentState object, so the request is empty.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommitmentStateRequest {
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommitmentStateResponse {
    /// The current CommitmentState
    #[prost(message, optional, tag="1")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateCommitmentStateRequest {
    /// The CommitmentState to set, must include complete state and pass validation checks.
    #[prost(message, optional, tag="1")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateCommitmentStateResponse {
    /// The result of the commitment state update.
    #[prost(message, optional, tag="1")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
include!("astria.execution.v1alpha2.tonic.rs");
// @@protoc_insertion_point(module)