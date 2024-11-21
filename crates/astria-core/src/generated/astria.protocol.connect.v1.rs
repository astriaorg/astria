#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtendedCommitInfoWithCurrencyPairMapping {
    #[prost(message, optional, tag = "1")]
    pub extended_commit_info: ::core::option::Option<
        super::super::super::super::astria_vendored::tendermint::abci::ExtendedCommitInfo,
    >,
    #[prost(message, repeated, tag = "2")]
    pub id_to_currency_pair: ::prost::alloc::vec::Vec<IdWithCurrencyPair>,
}
impl ::prost::Name for ExtendedCommitInfoWithCurrencyPairMapping {
    const NAME: &'static str = "ExtendedCommitInfoWithCurrencyPairMapping";
    const PACKAGE: &'static str = "astria.protocol.connect.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.connect.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IdWithCurrencyPair {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(message, optional, tag = "2")]
    pub currency_pair: ::core::option::Option<
        super::super::super::super::connect::types::v2::CurrencyPair,
    >,
}
impl ::prost::Name for IdWithCurrencyPair {
    const NAME: &'static str = "IdWithCurrencyPair";
    const PACKAGE: &'static str = "astria.protocol.connect.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.connect.v1.{}", Self::NAME)
    }
}
