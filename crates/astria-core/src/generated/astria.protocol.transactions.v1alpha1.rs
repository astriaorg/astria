/// `SignedTransaction` is a transaction that has
/// been signed by the given public key.
/// It wraps an `UnsignedTransaction` with a
/// signature and public key.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedTransaction {
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub transaction: ::core::option::Option<UnsignedTransaction>,
}
impl ::prost::Name for SignedTransaction {
    const NAME: &'static str = "SignedTransaction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `UnsignedTransaction` is a transaction that does
/// not have an attached signature.
/// Note: `value` must be set, it cannot be `None`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(message, repeated, tag = "1")]
    pub actions: ::prost::alloc::vec::Vec<Action>,
    #[prost(message, optional, tag = "2")]
    pub params: ::core::option::Option<TransactionParams>,
}
impl ::prost::Name for UnsignedTransaction {
    const NAME: &'static str = "UnsignedTransaction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `TransactionParams` contains parameters that define the
/// validity of the transaction.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionParams {
    #[prost(uint32, tag = "1")]
    pub nonce: u32,
    #[prost(string, tag = "2")]
    pub chain_id: ::prost::alloc::string::String,
}
impl ::prost::Name for TransactionParams {
    const NAME: &'static str = "TransactionParams";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(oneof = "action::Value", tags = "1, 2, 11, 12, 21, 22, 50, 51, 52, 53, 54")]
    pub value: ::core::option::Option<action::Value>,
}
/// Nested message and enum types in `Action`.
pub mod action {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        /// Core protocol actions are defined on 1-10
        #[prost(message, tag = "1")]
        TransferAction(super::TransferAction),
        #[prost(message, tag = "2")]
        SequenceAction(super::SequenceAction),
        /// Bridge actions are defined on 11-20
        #[prost(message, tag = "11")]
        InitBridgeAccountAction(super::InitBridgeAccountAction),
        #[prost(message, tag = "12")]
        BridgeLockAction(super::BridgeLockAction),
        /// IBC user actions are defined on 21-30
        #[prost(message, tag = "21")]
        IbcAction(::penumbra_proto::core::component::ibc::v1::IbcRelay),
        #[prost(message, tag = "22")]
        Ics20Withdrawal(super::Ics20Withdrawal),
        /// POA sudo actions are defined of 50-60
        #[prost(message, tag = "50")]
        SudoAddressChangeAction(super::SudoAddressChangeAction),
        #[prost(message, tag = "51")]
        ValidatorUpdateAction(::tendermint_proto::abci::ValidatorUpdate),
        #[prost(message, tag = "52")]
        IbcRelayerChangeAction(super::IbcRelayerChangeAction),
        #[prost(message, tag = "53")]
        FeeAssetChangeAction(super::FeeAssetChangeAction),
        #[prost(message, tag = "54")]
        MintAction(super::MintAction),
    }
}
impl ::prost::Name for Action {
    const NAME: &'static str = "Action";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `TransferAction` represents a value transfer transaction.
///
/// Note: all values must be set (ie. not `None`), otherwise it will
/// be considered invalid by the sequencer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransferAction {
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    /// the asset to be transferred
    #[prost(bytes = "vec", tag = "3")]
    pub asset_id: ::prost::alloc::vec::Vec<u8>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "4")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for TransferAction {
    const NAME: &'static str = "TransferAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `SequenceAction` represents a transaction destined for another
/// chain, ordered by the sequencer.
///
/// It contains the rollup ID of the destination chain, and the
/// opaque transaction data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequenceAction {
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::super::primitive::v1::RollupId>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SequenceAction {
    const NAME: &'static str = "SequenceAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// / `SudoAddressChangeAction` represents a transaction that changes
/// / the sudo address of the chain, which is the address authorized to
/// / make validator update actions.
/// /
/// / It contains the new sudo address.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SudoAddressChangeAction {
    #[prost(message, optional, tag = "1")]
    pub new_address: ::core::option::Option<super::super::super::primitive::v1::Address>,
}
impl ::prost::Name for SudoAddressChangeAction {
    const NAME: &'static str = "SudoAddressChangeAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `MintAction` represents a minting transaction.
/// It can only be executed by the chain's sudo address.
///
/// It contains the address to mint to, and the amount to mint.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MintAction {
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for MintAction {
    const NAME: &'static str = "MintAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ics20Withdrawal {
    /// first two fields are a transparent value consisting of an amount and a denom.
    #[prost(message, optional, tag = "1")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(string, tag = "2")]
    pub denom: ::prost::alloc::string::String,
    /// the address on the destination chain to send the transfer to.
    /// this is not validated by Astria; it is up to the destination chain
    /// to interpret it.
    #[prost(string, tag = "3")]
    pub destination_chain_address: ::prost::alloc::string::String,
    /// an Astria address to use to return funds from this withdrawal
    /// in the case it fails.
    #[prost(bytes = "vec", tag = "4")]
    pub return_address: ::prost::alloc::vec::Vec<u8>,
    /// the height (on Astria) at which this transfer expires.
    #[prost(message, optional, tag = "5")]
    pub timeout_height: ::core::option::Option<IbcHeight>,
    /// the unix timestamp (in nanoseconds) at which this transfer expires.
    #[prost(uint64, tag = "6")]
    pub timeout_time: u64,
    /// the source channel used for the withdrawal.
    #[prost(string, tag = "7")]
    pub source_channel: ::prost::alloc::string::String,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "8")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for Ics20Withdrawal {
    const NAME: &'static str = "Ics20Withdrawal";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcHeight {
    #[prost(uint64, tag = "1")]
    pub revision_number: u64,
    #[prost(uint64, tag = "2")]
    pub revision_height: u64,
}
impl ::prost::Name for IbcHeight {
    const NAME: &'static str = "IbcHeight";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `IbcRelayerChangeAction` represents a transaction that adds
/// or removes an IBC relayer address.
/// The bytes contained in each variant are the address to add or remove.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcRelayerChangeAction {
    #[prost(oneof = "ibc_relayer_change_action::Value", tags = "1, 2")]
    pub value: ::core::option::Option<ibc_relayer_change_action::Value>,
}
/// Nested message and enum types in `IbcRelayerChangeAction`.
pub mod ibc_relayer_change_action {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(message, tag = "1")]
        Addition(super::super::super::super::primitive::v1::Address),
        #[prost(message, tag = "2")]
        Removal(super::super::super::super::primitive::v1::Address),
    }
}
impl ::prost::Name for IbcRelayerChangeAction {
    const NAME: &'static str = "IbcRelayerChangeAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `FeeAssetChangeAction` represents a transaction that adds
/// or removes an asset for fee payments.
/// The bytes contained in each variant are the 32-byte asset ID
/// to add or remove.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeAssetChangeAction {
    #[prost(oneof = "fee_asset_change_action::Value", tags = "1, 2")]
    pub value: ::core::option::Option<fee_asset_change_action::Value>,
}
/// Nested message and enum types in `FeeAssetChangeAction`.
pub mod fee_asset_change_action {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(bytes, tag = "1")]
        Addition(::prost::alloc::vec::Vec<u8>),
        #[prost(bytes, tag = "2")]
        Removal(::prost::alloc::vec::Vec<u8>),
    }
}
impl ::prost::Name for FeeAssetChangeAction {
    const NAME: &'static str = "FeeAssetChangeAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `InitBridgeAccountAction` represents a transaction that initializes
/// a bridge account for the given rollup on the chain.
///
/// The sender of the transaction is used as the owner of the bridge account
/// and is the only actor authorized to transfer out of this account via
/// a `TransferAction`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InitBridgeAccountAction {
    /// the rollup ID to register with the bridge account (the tx sender)
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::super::primitive::v1::RollupId>,
    /// the asset IDs accepted as an incoming transfer by the bridge account
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub asset_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for InitBridgeAccountAction {
    const NAME: &'static str = "InitBridgeAccountAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `BridgeLockAction` represents a transaction that transfers
/// funds from a sequencer account to a bridge account.
///
/// It's the same as a `TransferAction` but with the added
/// `destination_chain_address` field.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeLockAction {
    /// the address of the bridge account to transfer to
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    /// the amount to transfer
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    /// the asset to be transferred
    #[prost(bytes = "vec", tag = "3")]
    pub asset_id: ::prost::alloc::vec::Vec<u8>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "4")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
    /// the address on the destination chain which
    /// will receive the bridged funds
    #[prost(string, tag = "5")]
    pub destination_chain_address: ::prost::alloc::string::String,
}
impl ::prost::Name for BridgeLockAction {
    const NAME: &'static str = "BridgeLockAction";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
