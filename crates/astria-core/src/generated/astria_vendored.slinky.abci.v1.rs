/// OracleVoteExtension defines the vote extension structure for oracle prices.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OracleVoteExtension {
    /// Prices defines a map of id(CurrencyPair) -> price.Bytes() . i.e. 1 ->
    /// 0x123.. (bytes). Notice the `id` function is determined by the
    /// `CurrencyPairIDStrategy` used in the VoteExtensionHandler.
    #[prost(btree_map = "uint64, bytes", tag = "1")]
    pub prices: ::prost::alloc::collections::BTreeMap<u64, ::prost::bytes::Bytes>,
}
impl ::prost::Name for OracleVoteExtension {
    const NAME: &'static str = "OracleVoteExtension";
    const PACKAGE: &'static str = "astria_vendored.slinky.abci.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.abci.v1.{}", Self::NAME)
    }
}
