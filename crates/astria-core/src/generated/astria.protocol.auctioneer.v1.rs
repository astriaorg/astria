#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnshrinedAuctioneerEntry {
    #[prost(message, optional, tag = "1")]
    pub auctioneer_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "2")]
    pub staker_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "3")]
    pub staked_amount: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(string, tag = "4")]
    pub fee_asset: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub asset: ::prost::alloc::string::String,
}
impl ::prost::Name for EnshrinedAuctioneerEntry {
    const NAME: &'static str = "EnshrinedAuctioneerEntry";
    const PACKAGE: &'static str = "astria.protocol.auctioneer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.auctioneer.v1.{}", Self::NAME)
    }
}
