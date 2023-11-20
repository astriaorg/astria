/// A response containing the balance of an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(message, optional, tag = "3")]
    pub balance: ::core::option::Option<super::super::primitive::v1::Uint128>,
}
/// A response containing the current nonce for an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NonceResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(uint32, tag = "3")]
    pub nonce: u32,
}
/// / Represents a denomination of some asset used within the sequencer.
/// / The `id` is used to identify the asset and for balance accounting.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Denom {
    #[prost(bytes = "vec", tag = "1")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "2")]
    pub base_denom: ::prost::alloc::string::String,
}
/// `IndexedTransaction` represents a sequencer transaction along with the index
/// it was originally in the sequencer block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IndexedTransaction {
    /// TODO: this is usize - how to define for variable size?
    #[prost(uint64, tag = "1")]
    pub block_index: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupNamespace {
    #[prost(uint64, tag = "1")]
    pub block_height: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub namespace: ::prost::alloc::vec::Vec<u8>,
}
/// `RollupNamespaceData`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupNamespaceData {
    #[prost(bytes = "vec", tag = "1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "2")]
    pub rollup_txs: ::prost::alloc::vec::Vec<IndexedTransaction>,
}
/// `SequencerNamespaceData`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerNamespaceData {
    #[prost(bytes = "vec", tag = "1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    #[prost(message, repeated, tag = "3")]
    pub sequencer_txs: ::prost::alloc::vec::Vec<IndexedTransaction>,
    #[prost(message, repeated, tag = "4")]
    pub rollup_namespaces: ::prost::alloc::vec::Vec<RollupNamespace>,
}
/// helper type - these should get parsed into a map from namespace to
/// a vector of `IndexedTransactions`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NamespacedIndexedTransactions {
    #[prost(bytes = "vec", tag = "1")]
    pub namespace: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "2")]
    pub txs: ::prost::alloc::vec::Vec<IndexedTransaction>,
}
/// `SequencerBlock`
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlock {
    #[prost(bytes = "vec", tag = "1")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    #[prost(message, repeated, tag = "3")]
    pub sequencer_transactions: ::prost::alloc::vec::Vec<IndexedTransaction>,
    /// FIXME: the current nested array layout results in bad allocation behavior on deserialization
    /// see <https://github.com/astriaorg/astria/issues/31>
    #[prost(message, repeated, tag = "4")]
    pub rollup_transactions: ::prost::alloc::vec::Vec<NamespacedIndexedTransactions>,
}
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
/// `UnsignedTransaction` is a transaction that does
/// not have an attached signature.
/// Note: `value` must be set, it cannot be `None`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(uint32, tag = "1")]
    pub nonce: u32,
    #[prost(message, repeated, tag = "2")]
    pub actions: ::prost::alloc::vec::Vec<Action>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(oneof = "action::Value", tags = "1, 2, 3, 4, 5, 6")]
    pub value: ::core::option::Option<action::Value>,
}
/// Nested message and enum types in `Action`.
pub mod action {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(message, tag = "1")]
        TransferAction(super::TransferAction),
        #[prost(message, tag = "2")]
        SequenceAction(super::SequenceAction),
        #[prost(message, tag = "3")]
        ValidatorUpdateAction(::tendermint_proto::abci::ValidatorUpdate),
        #[prost(message, tag = "4")]
        SudoAddressChangeAction(super::SudoAddressChangeAction),
        #[prost(message, tag = "5")]
        MintAction(super::MintAction),
        #[prost(message, tag = "6")]
        IbcAction(::penumbra_proto::core::component::ibc::v1alpha1::IbcAction),
    }
}
/// `TransferAction` represents a value transfer transaction.
///
/// Note: all values must be set (ie. not `None`), otherwise it will
/// be considered invalid by the sequencer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransferAction {
    #[prost(bytes = "vec", tag = "1")]
    pub to: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
    /// the asset to be transferred
    #[prost(bytes = "vec", tag = "3")]
    pub asset_id: ::prost::alloc::vec::Vec<u8>,
}
/// `SequenceAction` represents a transaction destined for another
/// chain, ordered by the sequencer.
///
/// It contains the chain ID of the destination chain, and the
/// opaque transaction data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequenceAction {
    #[prost(bytes = "vec", tag = "1")]
    pub chain_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// / `SudoAddressChangeAction` represents a transaction that changes
/// / the sudo address of the chain, which is the address authorized to
/// / make validator update actions.
/// /
/// / It contains the new sudo address.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SudoAddressChangeAction {
    #[prost(bytes = "vec", tag = "1")]
    pub new_address: ::prost::alloc::vec::Vec<u8>,
}
/// `MintAction` represents a minting transaction.
/// It can only be executed by the chain's sudo address.
///
/// It contains the address to mint to, and the amount to mint.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MintAction {
    #[prost(bytes = "vec", tag = "1")]
    pub to: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
}
