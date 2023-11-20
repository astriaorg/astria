// @generated
/// Request sent by the proposer to the relay to get the top of block bid.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTopOfBlockBidRequest {
    #[prost(uint64, tag="1")]
    pub block_height: u64,
}
/// Response sent by the relay to the proposer in response to a GetTopOfBlockBidRequest. Contains
/// the bid and hash of the top of block `SignedTransactions` for the proposer to commit to.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTopOfBlockBidResponse {
    #[prost(bytes="vec", tag="1")]
    pub builder_address: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
    #[prost(bytes="vec", tag="3")]
    pub payload_hash: ::prost::alloc::vec::Vec<u8>,
}
/// Request sent by the proposer to the relay to get the top of block payload. The commitment must be
/// a signature over the `payload_hash` the proposer received in `GetTopOfBlockResponse`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTopOfBlockPayloadRequest {
    #[prost(bytes="vec", tag="1")]
    pub builder_address: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="2")]
    pub block_height: u64,
    #[prost(bytes="vec", tag="3")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
}
/// Response sent by the relay to the proposer in response to a GetTopOfBlockPayloadRequest. Contains
/// the `SignedTransactions` payload for the top of block that the proposer committed to in `GetTopOfBlockPayloadRequest` .
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTopOfBlockPayloadResponse {
    #[prost(bytes="vec", tag="1")]
    pub builder_address: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag="3")]
    pub payload: ::core::option::Option<super::super::sequencer::v1alpha1::SignedTransaction>,
}
include!("astria.block_relay.v1alpha1.tonic.rs");
// @@protoc_insertion_point(module)