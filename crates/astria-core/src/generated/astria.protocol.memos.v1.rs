/// Memo for an ICS20 withdrawal from the rollup which is sent to
/// an external IBC-enabled chain.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ics20WithdrawalFromRollup {
    /// The block number on the rollup that triggered the transaction underlying
    /// this ics20 withdrawal memo.
    #[prost(uint64, tag = "1")]
    pub rollup_block_number: u64,
    /// An identifier of the original rollup withdrawal event that triggered this ics20
    /// withdrawal and that is underlying this bridge unlock memo. For general EVM
    /// this is typically a transaction hash.
    ///
    /// This field is of type `string` so that it can be formatted in the preferred
    /// format of the rollup when targeting plain text encoding.
    #[prost(string, tag = "2")]
    pub rollup_withdrawal_event_id: ::prost::alloc::string::String,
    /// The return address on the rollup to which funds should returned in case of
    /// failure. This field exists so that the rollup can identify which account
    /// the returned funds originated from.
    ///
    /// This field is of type `string` so that it can be formatted in the preferred
    /// format of the rollup when targeting plain text encoding.
    #[prost(string, tag = "3")]
    pub rollup_return_address: ::prost::alloc::string::String,
    /// A field that can be populated by the rollup. It is assumed that this field
    /// will be consumed by the downstream chain.
    #[prost(string, tag = "4")]
    pub memo: ::prost::alloc::string::String,
}
impl ::prost::Name for Ics20WithdrawalFromRollup {
    const NAME: &'static str = "Ics20WithdrawalFromRollup";
    const PACKAGE: &'static str = "astria.protocol.memos.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.memos.v1.{}", Self::NAME)
    }
}
/// Memo for an ICS20 transfer to Astria which is sent to a
/// bridge account, which will then be deposited into the rollup.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ics20TransferDeposit {
    /// The destination address for the deposit on the rollup.
    ///
    /// This field is of type `string` so that it can be formatted in the preferred
    /// format of the rollup when targeting plain text encoding.
    #[prost(string, tag = "1")]
    pub rollup_deposit_address: ::prost::alloc::string::String,
}
impl ::prost::Name for Ics20TransferDeposit {
    const NAME: &'static str = "Ics20TransferDeposit";
    const PACKAGE: &'static str = "astria.protocol.memos.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.memos.v1.{}", Self::NAME)
    }
}
