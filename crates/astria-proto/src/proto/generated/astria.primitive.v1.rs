// @generated
/// A 128 bit unsigned integer encoded in protobuf.,
///
/// Protobuf does not support integers larger than 64 bits,
/// so this message encodes a u128 by splitting it into its
/// upper 64 and lower 64 bits, each encoded as a uint64.
///
/// A native u128 x can then be constructed by casting both
/// integers to u128, left shifting hi by 64 positions and
/// adding lo:
///
/// x = (hi as u128) << 64 + (lo as u128)
#[derive(Copy)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Uint128 {
    #[prost(uint64, tag="1")]
    pub lo: u64,
    #[prost(uint64, tag="2")]
    pub hi: u64,
}
// @@protoc_insertion_point(module)
