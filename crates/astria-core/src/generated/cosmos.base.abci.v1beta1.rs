// This file is @generated by prost-build.
/// TxResponse defines a structure containing relevant tx data and metadata. The
/// tags are stringified and the log is JSON decoded.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TxResponse {
    /// The block height
    #[prost(int64, tag = "1")]
    pub height: i64,
    /// The transaction hash.
    #[prost(string, tag = "2")]
    pub txhash: ::prost::alloc::string::String,
    /// Namespace for the Code
    #[prost(string, tag = "3")]
    pub codespace: ::prost::alloc::string::String,
    /// Response code.
    #[prost(uint32, tag = "4")]
    pub code: u32,
    /// Result bytes, if any.
    #[prost(string, tag = "5")]
    pub data: ::prost::alloc::string::String,
    /// The output of the application's logger (raw string). May be
    /// non-deterministic.
    #[prost(string, tag = "6")]
    pub raw_log: ::prost::alloc::string::String,
    /// The output of the application's logger (typed). May be non-deterministic.
    #[prost(message, repeated, tag = "7")]
    pub logs: ::prost::alloc::vec::Vec<AbciMessageLog>,
    /// Additional information. May be non-deterministic.
    #[prost(string, tag = "8")]
    pub info: ::prost::alloc::string::String,
    /// Amount of gas requested for transaction.
    #[prost(int64, tag = "9")]
    pub gas_wanted: i64,
    /// Amount of gas consumed by transaction.
    #[prost(int64, tag = "10")]
    pub gas_used: i64,
    /// The request transaction bytes.
    #[prost(message, optional, tag = "11")]
    pub tx: ::core::option::Option<::pbjson_types::Any>,
    /// Time of the previous block. For heights > 1, it's the weighted median of
    /// the timestamps of the valid votes in the block.LastCommit. For height == 1,
    /// it's genesis time.
    #[prost(string, tag = "12")]
    pub timestamp: ::prost::alloc::string::String,
    /// Events defines all the events emitted by processing a transaction. Note,
    /// these events include those emitted by processing all the messages and those
    /// emitted from the ante. Whereas Logs contains the events, with
    /// additional metadata, emitted only by processing the messages.
    ///
    /// Since: cosmos-sdk 0.42.11, 0.44.5, 0.45
    #[prost(message, repeated, tag = "13")]
    pub events: ::prost::alloc::vec::Vec<
        super::super::super::super::tendermint::abci::Event,
    >,
}
impl ::prost::Name for TxResponse {
    const NAME: &'static str = "TxResponse";
    const PACKAGE: &'static str = "cosmos.base.abci.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.base.abci.v1beta1.TxResponse".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.base.abci.v1beta1.TxResponse".into()
    }
}
/// ABCIMessageLog defines a structure containing an indexed tx ABCI message log.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AbciMessageLog {
    #[prost(uint32, tag = "1")]
    pub msg_index: u32,
    #[prost(string, tag = "2")]
    pub log: ::prost::alloc::string::String,
    /// Events contains a slice of Event objects that were emitted during some
    /// execution.
    #[prost(message, repeated, tag = "3")]
    pub events: ::prost::alloc::vec::Vec<StringEvent>,
}
impl ::prost::Name for AbciMessageLog {
    const NAME: &'static str = "ABCIMessageLog";
    const PACKAGE: &'static str = "cosmos.base.abci.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.base.abci.v1beta1.ABCIMessageLog".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.base.abci.v1beta1.ABCIMessageLog".into()
    }
}
/// StringEvent defines en Event object wrapper where all the attributes
/// contain key/value pairs that are strings instead of raw bytes.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StringEvent {
    #[prost(string, tag = "1")]
    pub r#type: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub attributes: ::prost::alloc::vec::Vec<Attribute>,
}
impl ::prost::Name for StringEvent {
    const NAME: &'static str = "StringEvent";
    const PACKAGE: &'static str = "cosmos.base.abci.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.base.abci.v1beta1.StringEvent".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.base.abci.v1beta1.StringEvent".into()
    }
}
/// Attribute defines an attribute wrapper where the key and value are
/// strings instead of raw bytes.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Attribute {
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub value: ::prost::alloc::string::String,
}
impl ::prost::Name for Attribute {
    const NAME: &'static str = "Attribute";
    const PACKAGE: &'static str = "cosmos.base.abci.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.base.abci.v1beta1.Attribute".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.base.abci.v1beta1.Attribute".into()
    }
}
