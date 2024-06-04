/// A response containing the denomination given an asset ID.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DenomResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(string, tag = "3")]
    pub denom: ::prost::alloc::string::String,
}
impl ::prost::Name for DenomResponse {
    const NAME: &'static str = "DenomResponse";
    const PACKAGE: &'static str = "astria.protocol.asset.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.asset.v1alpha1.{}", Self::NAME)
    }
}
/// A response containing the allowed fee asset IDs.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllowedFeeAssetIdsResponse {
    #[prost(uint64, tag = "1")]
    pub height: u64,
    #[prost(bytes = "bytes", repeated, tag = "2")]
    pub fee_asset_ids: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
impl ::prost::Name for AllowedFeeAssetIdsResponse {
    const NAME: &'static str = "AllowedFeeAssetIdsResponse";
    const PACKAGE: &'static str = "astria.protocol.asset.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.asset.v1alpha1.{}", Self::NAME)
    }
}
