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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtendedCommitInfo {
    /// The round at which the block proposer decided in the previous height.
    #[prost(int32, tag = "1")]
    pub round: i32,
    /// List of validators' addresses in the last validator set with their voting
    /// information, including vote extensions.
    #[prost(message, repeated, tag = "2")]
    pub votes: ::prost::alloc::vec::Vec<ExtendedVoteInfo>,
}
impl ::prost::Name for ExtendedCommitInfo {
    const NAME: &'static str = "ExtendedCommitInfo";
    const PACKAGE: &'static str = "astria_vendored.tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.abci.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExtendedVoteInfo {
    /// The validator that sent the vote.
    #[prost(message, optional, tag = "1")]
    pub validator: ::core::option::Option<Validator>,
    /// Non-deterministic extension provided by the sending validator's application.
    #[prost(bytes = "bytes", tag = "3")]
    pub vote_extension: ::prost::bytes::Bytes,
    /// Vote extension signature created by CometBFT
    #[prost(bytes = "bytes", tag = "4")]
    pub extension_signature: ::prost::bytes::Bytes,
    /// block_id_flag indicates whether the validator voted for a block, nil, or did not vote at all
    #[prost(enumeration = "::tendermint_proto::types::BlockIdFlag", tag = "5")]
    pub block_id_flag: i32,
}
impl ::prost::Name for ExtendedVoteInfo {
    const NAME: &'static str = "ExtendedVoteInfo";
    const PACKAGE: &'static str = "astria_vendored.tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.abci.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Validator {
    #[prost(bytes = "bytes", tag = "1")]
    pub address: ::prost::bytes::Bytes,
    #[prost(int64, tag = "3")]
    pub power: i64,
}
impl ::prost::Name for Validator {
    const NAME: &'static str = "Validator";
    const PACKAGE: &'static str = "astria_vendored.tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.tendermint.abci.{}", Self::NAME)
    }
}
