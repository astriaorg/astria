#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorUpdate {
    #[prost(message, optional, tag = "1")]
    pub pub_key: ::core::option::Option<super::crypto::PublicKey>,
    #[prost(int64, tag = "2")]
    pub power: i64,
}
impl ::prost::Name for ValidatorUpdate {
    const NAME: &'static str = "ValidatorUpdate";
    const PACKAGE: &'static str = "astria_vendored.tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.abci.{}", Self::NAME)
    }
}
