#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StakedBuilderEntry {
    #[prost(message, optional, tag = "1")]
    pub creator_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "2")]
    pub builder_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "3")]
    pub staked_amount: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(string, tag = "4")]
    pub asset: ::prost::alloc::string::String,
}
impl ::prost::Name for StakedBuilderEntry {
    const NAME: &'static str = "StakedBuilderEntry";
    const PACKAGE: &'static str = "astria.protocol.enshrinedbuilder.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.protocol.enshrinedbuilder.v1alpha1.{}", Self::NAME
        )
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnstakedBuilderEntry {
    #[prost(message, optional, tag = "1")]
    pub creator_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "2")]
    pub builder_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "3")]
    pub time: ::core::option::Option<::pbjson_types::Timestamp>,
    #[prost(string, tag = "4")]
    pub asset: ::prost::alloc::string::String,
}
impl ::prost::Name for UnstakedBuilderEntry {
    const NAME: &'static str = "UnstakedBuilderEntry";
    const PACKAGE: &'static str = "astria.protocol.enshrinedbuilder.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.protocol.enshrinedbuilder.v1alpha1.{}", Self::NAME
        )
    }
}
