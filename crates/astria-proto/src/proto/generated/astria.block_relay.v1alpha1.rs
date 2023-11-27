// @generated
/// `SignedBid` is a bid that has been signed by the given public key.
/// It wraps a `BidPayload` with a signature and public key.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedBundle {
    #[prost(bytes="vec", tag="1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub bundle: ::core::option::Option<Bundle>,
}
/// `BidPayload` is a bid without the signature and associated public key. It is a wrapper
/// around an `UnsignedTransaction` that adds a 
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Bundle {
    /// The block height for which this bid is valid
    #[prost(uint64, tag="1")]
    pub block_height: u64,
    /// The transfer action for paying the bid amount
    #[prost(message, optional, tag="2")]
    pub bid: ::core::option::Option<super::super::sequencer::v1alpha1::TransferAction>,
    /// The transaction that is being bid on.
    #[prost(message, optional, tag="3")]
    pub bundle: ::core::option::Option<super::super::sequencer::v1alpha1::UnsignedTransaction>,
}
/// `OpaqueBid` is a bid and the hash of the payload. It is revealed by the Relay to the Proposer
/// for commitment before the payload is shared with the Proposer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpaqueBid {
    /// The block height for which this bid is valid
    #[prost(uint64, tag="1")]
    pub block_height: u64,
    /// The transfer action for paying the bid amount
    #[prost(message, optional, tag="2")]
    pub bid: ::core::option::Option<super::super::sequencer::v1alpha1::TransferAction>,
    /// The hash of the transaction that is being bid on.
    #[prost(bytes="vec", tag="3")]
    pub payload_hash: ::prost::alloc::vec::Vec<u8>,
}
/// Request sent by the Proposer to the Relay to get the top of block bid.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBidRequest {
    #[prost(uint64, tag="1")]
    pub block_height: u64,
}
/// Response sent by the Relay to the Proposer in response to a GetBidRequest. Contains
/// the bid amount and hash of the `Bundle` for the Proposer to commit to.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBidResponse {
    #[prost(message, optional, tag="1")]
    pub bid: ::core::option::Option<OpaqueBid>,
}
/// Request sent by the Proposer to the Relay to get the top of block bundle. The commitment must be
/// a signature over the `payload_hash` the Proposer received in `GetBidResponse`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBundleRequest {
    #[prost(message, optional, tag="1")]
    pub bid: ::core::option::Option<OpaqueBid>,
    #[prost(bytes="vec", tag="2")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
}
/// Response sent by the Relay to the Proposer in response to a GetBundleRequest. Contains
/// the `Bundle` payload for the top of block that the Proposer committed to in `GetBundleRequest`
/// as well as the bidder's signature and public key for verification.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBundleResponse {
    #[prost(message, optional, tag="1")]
    pub bundle: ::core::option::Option<SignedBundle>,
}
include!("astria.block_relay.v1alpha1.tonic.rs");
// @@protoc_insertion_point(module)