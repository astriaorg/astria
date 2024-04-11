/// Event allows application developers to attach additional information to
/// ResponseBeginBlock, ResponseEndBlock, ResponseCheckTx and ResponseDeliverTx.
/// Later, transactions may be queried using these events.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Event {
    #[prost(string, tag = "1")]
    pub r#type: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub attributes: ::prost::alloc::vec::Vec<EventAttribute>,
}
impl ::prost::Name for Event {
    const NAME: &'static str = "Event";
    const PACKAGE: &'static str = "tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("tendermint.abci.{}", Self::NAME)
    }
}
/// EventAttribute is a single key-value pair, associated with an event.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EventAttribute {
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    /// nondeterministic
    #[prost(bool, tag = "3")]
    pub index: bool,
}
impl ::prost::Name for EventAttribute {
    const NAME: &'static str = "EventAttribute";
    const PACKAGE: &'static str = "tendermint.abci";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("tendermint.abci.{}", Self::NAME)
    }
}
