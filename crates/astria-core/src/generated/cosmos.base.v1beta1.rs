/// Coin defines a token with a denomination and an amount.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}
impl ::prost::Name for Coin {
    const NAME: &'static str = "Coin";
    const PACKAGE: &'static str = "cosmos.base.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("cosmos.base.v1beta1.{}", Self::NAME)
    }
}
