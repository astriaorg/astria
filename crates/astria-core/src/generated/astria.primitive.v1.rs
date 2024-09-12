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
    #[prost(uint64, tag = "1")]
    pub lo: u64,
    #[prost(uint64, tag = "2")]
    pub hi: u64,
}
impl ::prost::Name for Uint128 {
    const NAME: &'static str = "Uint128";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
/// A proof for a tree of the given size containing the audit path from a leaf to the root.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Proof {
    /// A sequence of 32 byte hashes used to reconstruct a Merkle Tree Hash.
    #[prost(bytes = "bytes", tag = "1")]
    pub audit_path: ::prost::bytes::Bytes,
    /// The index of the leaf this proof applies to.
    #[prost(uint64, tag = "2")]
    pub leaf_index: u64,
    /// The total size of the tree this proof was derived from.
    #[prost(uint64, tag = "3")]
    pub tree_size: u64,
}
impl ::prost::Name for Proof {
    const NAME: &'static str = "Proof";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
/// / Represents a denomination of some asset used within the sequencer.
/// / The `id` is used to identify the asset and for balance accounting.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Denom {
    #[prost(bytes = "bytes", tag = "1")]
    pub id: ::prost::bytes::Bytes,
    #[prost(string, tag = "2")]
    pub base_denom: ::prost::alloc::string::String,
}
impl ::prost::Name for Denom {
    const NAME: &'static str = "Denom";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
/// A `RollupId` is a unique identifier for a rollup chain.
/// It must be 32 bytes long. It can be derived from a string
/// using a sha256 hash.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupId {
    #[prost(bytes = "bytes", tag = "1")]
    pub inner: ::prost::bytes::Bytes,
}
impl ::prost::Name for RollupId {
    const NAME: &'static str = "RollupId";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
/// An Astria `Address`.
///
/// Astria addresses are bech32m encoded strings, with the data part being the
/// first 20 entries of a sha256-hashed ed25519 public key.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Address {
    /// A bech32m encoded string. The data are the first 20 bytes of a sha256-hashed ed25519
    /// public key. Implementors must not accept both the `bytes` and `bech32m` being set.
    #[prost(string, tag = "2")]
    pub bech32m: ::prost::alloc::string::String,
}
impl ::prost::Name for Address {
    const NAME: &'static str = "Address";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
/// A `TransactionId` is a unique identifier for a transaction.
/// It contains the hash of the transaction, to be included in
/// rollup deposit events for source tracking.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionId {
    /// The hash of the transaction which the ID represents.
    /// It must be a lower hex-encoded 32-byte hash.
    #[prost(string, tag = "1")]
    pub inner: ::prost::alloc::string::String,
}
impl ::prost::Name for TransactionId {
    const NAME: &'static str = "TransactionId";
    const PACKAGE: &'static str = "astria.primitive.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.primitive.v1.{}", Self::NAME)
    }
}
