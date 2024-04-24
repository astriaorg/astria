#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtocolVersion {
    #[prost(uint64, tag = "1")]
    pub p2p: u64,
    #[prost(uint64, tag = "2")]
    pub block: u64,
    #[prost(uint64, tag = "3")]
    pub app: u64,
}
impl ::prost::Name for ProtocolVersion {
    const NAME: &'static str = "ProtocolVersion";
    const PACKAGE: &'static str = "tendermint.p2p";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("tendermint.p2p.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DefaultNodeInfo {
    #[prost(message, optional, tag = "1")]
    pub protocol_version: ::core::option::Option<ProtocolVersion>,
    #[prost(string, tag = "2")]
    pub default_node_id: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub listen_addr: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub network: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub version: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "6")]
    pub channels: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "7")]
    pub moniker: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "8")]
    pub other: ::core::option::Option<DefaultNodeInfoOther>,
}
impl ::prost::Name for DefaultNodeInfo {
    const NAME: &'static str = "DefaultNodeInfo";
    const PACKAGE: &'static str = "tendermint.p2p";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("tendermint.p2p.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DefaultNodeInfoOther {
    #[prost(string, tag = "1")]
    pub tx_index: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub rpc_address: ::prost::alloc::string::String,
}
impl ::prost::Name for DefaultNodeInfoOther {
    const NAME: &'static str = "DefaultNodeInfoOther";
    const PACKAGE: &'static str = "tendermint.p2p";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("tendermint.p2p.{}", Self::NAME)
    }
}
