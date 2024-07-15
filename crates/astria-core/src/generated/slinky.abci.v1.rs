/// OracleVoteExtension defines the vote extension structure for oracle prices.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OracleVoteExtension {
    /// Prices defines a map of id(CurrencyPair) -> price.Bytes() . i.e. 1 ->
    /// 0x123.. (bytes). Notice the `id` function is determined by the
    /// `CurrencyPairIDStrategy` used in the VoteExtensionHandler.
    #[prost(map = "uint64, bytes", tag = "1")]
    pub prices: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for OracleVoteExtension {
    const NAME: &'static str = "OracleVoteExtension";
    const PACKAGE: &'static str = "slinky.abci.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.abci.v1.{}", Self::NAME)
    }
}
