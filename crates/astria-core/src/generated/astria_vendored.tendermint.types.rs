#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockId {
    #[prost(bytes = "vec", tag = "1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub part_set_header: ::core::option::Option<PartSetHeader>,
}
impl ::prost::Name for BlockId {
    const NAME: &'static str = "BlockID";
    const PACKAGE: &'static str = "astria_vendored.tendermint.types";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.types.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PartSetHeader {
    #[prost(uint32, tag = "1")]
    pub total: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for PartSetHeader {
    const NAME: &'static str = "PartSetHeader";
    const PACKAGE: &'static str = "astria_vendored.tendermint.types";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.types.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Header {
    /// basic block info
    #[prost(message, optional, tag = "1")]
    pub version: ::core::option::Option<super::version::Consensus>,
    #[prost(string, tag = "2")]
    pub chain_id: ::prost::alloc::string::String,
    #[prost(int64, tag = "3")]
    pub height: i64,
    #[prost(message, optional, tag = "4")]
    pub time: ::core::option::Option<::pbjson_types::Timestamp>,
    /// prev block info
    #[prost(message, optional, tag = "5")]
    pub last_block_id: ::core::option::Option<BlockId>,
    /// hashes of block data
    #[prost(bytes = "vec", tag = "6")]
    pub last_commit_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub data_hash: ::prost::alloc::vec::Vec<u8>,
    /// hashes from the app output from the prev block
    #[prost(bytes = "vec", tag = "8")]
    pub validators_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "9")]
    pub next_validators_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "10")]
    pub consensus_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "11")]
    pub app_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "12")]
    pub last_results_hash: ::prost::alloc::vec::Vec<u8>,
    /// consensus info
    #[prost(bytes = "vec", tag = "13")]
    pub evidence_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "14")]
    pub proposer_address: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for Header {
    const NAME: &'static str = "Header";
    const PACKAGE: &'static str = "astria_vendored.tendermint.types";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.types.{}", Self::NAME)
    }
}
