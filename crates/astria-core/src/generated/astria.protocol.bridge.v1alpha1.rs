/// A response containing the last tx hash given some bridge address,
/// if it exists.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeAccountLastTxHashResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(bytes = "vec", optional, tag = "3")]
    pub tx_hash: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for BridgeAccountLastTxHashResponse {
    const NAME: &'static str = "BridgeAccountLastTxHashResponse";
    const PACKAGE: &'static str = "astria.protocol.bridge.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.bridge.v1alpha1.{}", Self::NAME)
    }
}
