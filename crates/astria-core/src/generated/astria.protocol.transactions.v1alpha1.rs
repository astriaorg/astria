#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(
        oneof = "action::Value",
        tags = "1, 2, 11, 12, 13, 14, 21, 22, 50, 51, 52, 53, 55, 56"
    )]
    pub value: ::core::option::Option<action::Value>,
}
/// Nested message and enum types in `Action`.
pub mod action {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        /// Core protocol actions are defined on 1-10
        #[prost(message, tag = "1")]
        Transfer(super::Transfer),
        #[prost(message, tag = "2")]
        Sequence(super::Sequence),
        /// Bridge actions are defined on 11-20
        #[prost(message, tag = "11")]
        InitBridgeAccount(super::InitBridgeAccount),
        #[prost(message, tag = "12")]
        BridgeLock(super::BridgeLock),
        #[prost(message, tag = "13")]
        BridgeUnlock(super::BridgeUnlock),
        #[prost(message, tag = "14")]
        BridgeSudoChange(super::BridgeSudoChange),
        /// IBC user actions are defined on 21-30
        #[prost(message, tag = "21")]
        Ibc(::penumbra_proto::core::component::ibc::v1::IbcRelay),
        #[prost(message, tag = "22")]
        Ics20Withdrawal(super::Ics20Withdrawal),
        /// POA sudo actions are defined on 50-60
        #[prost(message, tag = "50")]
        SudoAddressChange(super::SudoAddressChange),
        #[prost(message, tag = "51")]
        ValidatorUpdate(
            crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate,
        ),
        #[prost(message, tag = "52")]
        IbcRelayerChange(super::IbcRelayerChange),
        #[prost(message, tag = "53")]
        FeeAssetChange(super::FeeAssetChange),
        #[prost(message, tag = "55")]
        FeeChange(super::FeeChange),
        #[prost(message, tag = "56")]
        IbcSudoChange(super::IbcSudoChange),
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
pub struct Transfer {
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    /// the asset to be transferred
    #[prost(string, tag = "3")]
    pub asset: ::prost::alloc::string::String,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "4")]
    pub fee_asset: ::prost::alloc::string::String,
}
impl ::prost::Name for Transfer {
    const NAME: &'static str = "Transfer";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `Sequence` represents a transaction destined for another
/// chain, ordered by the sequencer.
///
/// It contains the rollup ID of the destination chain, and the
/// opaque transaction data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Sequence {
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::super::primitive::v1::RollupId>,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: ::prost::bytes::Bytes,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "3")]
    pub fee_asset: ::prost::alloc::string::String,
}
impl ::prost::Name for Sequence {
    const NAME: &'static str = "Sequence";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// / `SudoAddressChange` represents a transaction that changes
/// / the sudo address of the chain, which is the address authorized to
/// / make validator update actions.
/// /
/// / It contains the new sudo address.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SudoAddressChange {
    #[prost(message, optional, tag = "1")]
    pub new_address: ::core::option::Option<super::super::super::primitive::v1::Address>,
}
impl ::prost::Name for SudoAddressChange {
    const NAME: &'static str = "SudoAddressChange";
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
    #[prost(message, optional, tag = "4")]
    pub return_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
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
    #[prost(string, tag = "8")]
    pub fee_asset: ::prost::alloc::string::String,
    /// a memo to include with the transfer
    #[prost(string, tag = "9")]
    pub memo: ::prost::alloc::string::String,
    /// the address of the bridge account to transfer from, if this is a withdrawal
    /// from a bridge account and the sender of the tx is the bridge's withdrawer,
    /// which differs from the bridge account's address.
    ///
    /// if unset, and the transaction sender is not a bridge account, the withdrawal
    /// is treated as a user (non-bridge) withdrawal.
    ///
    /// if unset, and the transaction sender is a bridge account, the withdrawal is
    /// treated as a bridge withdrawal (ie. the bridge account's withdrawer address is checked).
    #[prost(message, optional, tag = "10")]
    pub bridge_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// whether to use a bech32-compatible format of the `.return_address` when generating
    /// fungible token packets (as opposed to Astria-native bech32m addresses). This is
    /// necessary for chains like noble which enforce a strict bech32 format.
    #[prost(bool, tag = "11")]
    pub use_compat_address: bool,
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
/// `IbcRelayerChange` represents a transaction that adds
/// or removes an IBC relayer address.
/// The bytes contained in each variant are the address to add or remove.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcRelayerChange {
    #[prost(oneof = "ibc_relayer_change::Value", tags = "1, 2")]
    pub value: ::core::option::Option<ibc_relayer_change::Value>,
}
/// Nested message and enum types in `IbcRelayerChange`.
pub mod ibc_relayer_change {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(message, tag = "1")]
        Addition(super::super::super::super::primitive::v1::Address),
        #[prost(message, tag = "2")]
        Removal(super::super::super::super::primitive::v1::Address),
    }
}
impl ::prost::Name for IbcRelayerChange {
    const NAME: &'static str = "IbcRelayerChange";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `FeeAssetChange` represents a transaction that adds
/// or removes an asset for fee payments.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeAssetChange {
    #[prost(oneof = "fee_asset_change::Value", tags = "1, 2")]
    pub value: ::core::option::Option<fee_asset_change::Value>,
}
/// Nested message and enum types in `FeeAssetChange`.
pub mod fee_asset_change {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(string, tag = "1")]
        Addition(::prost::alloc::string::String),
        #[prost(string, tag = "2")]
        Removal(::prost::alloc::string::String),
    }
}
impl ::prost::Name for FeeAssetChange {
    const NAME: &'static str = "FeeAssetChange";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `InitBridgeAccount` represents a transaction that initializes
/// a bridge account for the given rollup on the chain.
///
/// The sender of the transaction is used as the owner of the bridge account
/// and is the only actor authorized to transfer out of this account via
/// a `Transfer`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InitBridgeAccount {
    /// the rollup ID to register with the bridge account (the tx sender)
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::super::primitive::v1::RollupId>,
    /// the asset ID accepted as an incoming transfer by the bridge account
    #[prost(string, tag = "2")]
    pub asset: ::prost::alloc::string::String,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "3")]
    pub fee_asset: ::prost::alloc::string::String,
    /// the address corresponding to the key which has sudo capabilities;
    /// ie. can change the sudo and withdrawer addresses for this bridge account.
    /// if this is empty, the sender of the transaction is used.
    #[prost(message, optional, tag = "4")]
    pub sudo_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// the address corresponding to the key which can withdraw funds from this bridge account.
    /// if this is empty, the sender of the transaction is used.
    #[prost(message, optional, tag = "5")]
    pub withdrawer_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
}
impl ::prost::Name for InitBridgeAccount {
    const NAME: &'static str = "InitBridgeAccount";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `BridgeLock` represents a transaction that transfers
/// funds from a sequencer account to a bridge account.
///
/// It's the same as a `Transfer` but with the added
/// `destination_chain_address` field.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeLock {
    /// the address of the bridge account to transfer to
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    /// the amount to transfer
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    /// the asset to be transferred
    #[prost(string, tag = "3")]
    pub asset: ::prost::alloc::string::String,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "4")]
    pub fee_asset: ::prost::alloc::string::String,
    /// the address on the destination chain which
    /// will receive the bridged funds
    #[prost(string, tag = "5")]
    pub destination_chain_address: ::prost::alloc::string::String,
}
impl ::prost::Name for BridgeLock {
    const NAME: &'static str = "BridgeLock";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `BridgeUnlock` represents a transaction that transfers
/// funds from a bridge account to a sequencer account.
///
/// It's the same as a `Transfer` but without the `asset` field
/// and with the `memo` field.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeUnlock {
    /// the to withdraw funds to
    #[prost(message, optional, tag = "1")]
    pub to: ::core::option::Option<super::super::super::primitive::v1::Address>,
    /// the amount to transfer
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "3")]
    pub fee_asset: ::prost::alloc::string::String,
    /// The memo field can be used to provide unique identifying additional
    /// information about the bridge unlock transaction.
    #[prost(string, tag = "4")]
    pub memo: ::prost::alloc::string::String,
    /// the address of the bridge account to transfer from
    #[prost(message, optional, tag = "5")]
    pub bridge_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// The block number on the rollup that triggered the transaction underlying
    /// this bridge unlock memo.
    #[prost(uint64, tag = "6")]
    pub rollup_block_number: u64,
    /// An identifier of the original rollup event, such as a transaction hash which
    /// triggered a bridge unlock and is underlying event that led to this bridge
    /// unlock. This can be utilized for tracing from the bridge back to
    /// distinct rollup events.
    ///
    /// This field is of type `string` so that it can be formatted in the preferred
    /// format of the rollup when targeting plain text encoding.
    #[prost(string, tag = "7")]
    pub rollup_withdrawal_event_id: ::prost::alloc::string::String,
}
impl ::prost::Name for BridgeUnlock {
    const NAME: &'static str = "BridgeUnlock";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeSudoChange {
    /// the address of the bridge account to change the sudo or withdrawer addresses for
    #[prost(message, optional, tag = "1")]
    pub bridge_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// the new sudo address; unchanged if unset
    #[prost(message, optional, tag = "2")]
    pub new_sudo_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// the new withdrawer address; unchanged if unset
    #[prost(message, optional, tag = "3")]
    pub new_withdrawer_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    /// the asset used to pay the transaction fee
    #[prost(string, tag = "4")]
    pub fee_asset: ::prost::alloc::string::String,
}
impl ::prost::Name for BridgeSudoChange {
    const NAME: &'static str = "BridgeSudoChange";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeChange {
    /// the new fee components values
    #[prost(
        oneof = "fee_change::FeeComponents",
        tags = "1, 2, 3, 4, 5, 7, 6, 8, 9, 10, 11, 12, 13, 14"
    )]
    pub fee_components: ::core::option::Option<fee_change::FeeComponents>,
}
/// Nested message and enum types in `FeeChange`.
pub mod fee_change {
    /// the new fee components values
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum FeeComponents {
        #[prost(message, tag = "1")]
        BridgeLock(super::super::super::fees::v1alpha1::BridgeLockFeeComponents),
        #[prost(message, tag = "2")]
        BridgeSudoChange(
            super::super::super::fees::v1alpha1::BridgeSudoChangeFeeComponents,
        ),
        #[prost(message, tag = "3")]
        BridgeUnlock(super::super::super::fees::v1alpha1::BridgeUnlockFeeComponents),
        #[prost(message, tag = "4")]
        FeeAssetChange(super::super::super::fees::v1alpha1::FeeAssetChangeFeeComponents),
        #[prost(message, tag = "5")]
        FeeChange(super::super::super::fees::v1alpha1::FeeChangeFeeComponents),
        #[prost(message, tag = "7")]
        IbcRelay(super::super::super::fees::v1alpha1::IbcRelayFeeComponents),
        #[prost(message, tag = "6")]
        IbcRelayerChange(
            super::super::super::fees::v1alpha1::IbcRelayerChangeFeeComponents,
        ),
        #[prost(message, tag = "8")]
        IbcSudoChange(super::super::super::fees::v1alpha1::IbcSudoChangeFeeComponents),
        #[prost(message, tag = "9")]
        Ics20Withdrawal(
            super::super::super::fees::v1alpha1::Ics20WithdrawalFeeComponents,
        ),
        #[prost(message, tag = "10")]
        InitBridgeAccount(
            super::super::super::fees::v1alpha1::InitBridgeAccountFeeComponents,
        ),
        #[prost(message, tag = "11")]
        Sequence(super::super::super::fees::v1alpha1::SequenceFeeComponents),
        #[prost(message, tag = "12")]
        SudoAddressChange(
            super::super::super::fees::v1alpha1::SudoAddressChangeFeeComponents,
        ),
        #[prost(message, tag = "13")]
        Transfer(super::super::super::fees::v1alpha1::TransferFeeComponents),
        #[prost(message, tag = "14")]
        ValidatorUpdate(
            super::super::super::fees::v1alpha1::ValidatorUpdateFeeComponents,
        ),
    }
}
impl ::prost::Name for FeeChange {
    const NAME: &'static str = "FeeChange";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcSudoChange {
    #[prost(message, optional, tag = "1")]
    pub new_address: ::core::option::Option<super::super::super::primitive::v1::Address>,
}
impl ::prost::Name for IbcSudoChange {
    const NAME: &'static str = "IbcSudoChange";
    const PACKAGE: &'static str = "astria.protocol.transactions.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.transactions.v1alpha1.{}", Self::NAME)
    }
}
/// `SignedTransaction` is a transaction that has
/// been signed by the given public key.
/// It wraps an `UnsignedTransaction` with a
/// signature and public key.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedTransaction {
    #[prost(bytes = "bytes", tag = "1")]
    pub signature: ::prost::bytes::Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub public_key: ::prost::bytes::Bytes,
    #[prost(message, optional, tag = "3")]
    pub transaction: ::core::option::Option<::pbjson_types::Any>,
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
