/// CurrencyPair is the standard representation of a pair of assets, where one
/// (Base) is priced in terms of the other (Quote)
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrencyPair {
    #[prost(string, tag = "1")]
    pub base: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub quote: ::prost::alloc::string::String,
}
impl ::prost::Name for CurrencyPair {
    const NAME: &'static str = "CurrencyPair";
    const PACKAGE: &'static str = "astria_vendored.connect.types.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.connect.types.v2.{}", Self::NAME)
    }
}
