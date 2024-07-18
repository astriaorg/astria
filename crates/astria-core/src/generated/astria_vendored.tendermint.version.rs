#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Consensus {
    #[prost(uint64, tag = "1")]
    pub block: u64,
    #[prost(uint64, tag = "2")]
    pub app: u64,
}
impl ::prost::Name for Consensus {
    const NAME: &'static str = "Consensus";
    const PACKAGE: &'static str = "astria_vendored.tendermint.version";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.version.{}", Self::NAME)
    }
}
