#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AssetBalance {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub balance: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for AssetBalance {
    const NAME: &'static str = "AssetBalance";
    const PACKAGE: &'static str = "astria.protocol.accounts.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.accounts.v1alpha1.{}", Self::NAME)
    }
}
/// A response containing the balance of an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(message, repeated, tag = "3")]
    pub balances: ::prost::alloc::vec::Vec<AssetBalance>,
}
impl ::prost::Name for BalanceResponse {
    const NAME: &'static str = "BalanceResponse";
    const PACKAGE: &'static str = "astria.protocol.accounts.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.accounts.v1alpha1.{}", Self::NAME)
    }
}
/// A response containing the current nonce for an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NonceResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(uint32, tag = "3")]
    pub nonce: u32,
}
impl ::prost::Name for NonceResponse {
    const NAME: &'static str = "NonceResponse";
    const PACKAGE: &'static str = "astria.protocol.accounts.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.accounts.v1alpha1.{}", Self::NAME)
    }
}
