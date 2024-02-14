#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum AbciErrorCode {
    Unspecified = 0,
    UnknownPath = 1,
    InvalidParameter = 2,
    InternalError = 3,
    InvalidNonce = 4,
    TransactionTooLarge = 5,
}
impl AbciErrorCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            AbciErrorCode::Unspecified => "ABCI_ERROR_CODE_UNSPECIFIED",
            AbciErrorCode::UnknownPath => "ABCI_ERROR_CODE_UNKNOWN_PATH",
            AbciErrorCode::InvalidParameter => "ABCI_ERROR_CODE_INVALID_PARAMETER",
            AbciErrorCode::InternalError => "ABCI_ERROR_CODE_INTERNAL_ERROR",
            AbciErrorCode::InvalidNonce => "ABCI_ERROR_CODE_INVALID_NONCE",
            AbciErrorCode::TransactionTooLarge => "ABCI_ERROR_CODE_TRANSACTION_TOO_LARGE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "ABCI_ERROR_CODE_UNSPECIFIED" => Some(Self::Unspecified),
            "ABCI_ERROR_CODE_UNKNOWN_PATH" => Some(Self::UnknownPath),
            "ABCI_ERROR_CODE_INVALID_PARAMETER" => Some(Self::InvalidParameter),
            "ABCI_ERROR_CODE_INTERNAL_ERROR" => Some(Self::InternalError),
            "ABCI_ERROR_CODE_INVALID_NONCE" => Some(Self::InvalidNonce),
            "ABCI_ERROR_CODE_TRANSACTION_TOO_LARGE" => Some(Self::TransactionTooLarge),
            _ => None,
        }
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AssetBalance {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub balance: ::core::option::Option<super::super::primitive::v1::Uint128>,
}
/// A response containing the balance of an account.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(message, repeated, tag = "3")]
    pub balances: ::prost::alloc::vec::Vec<AssetBalance>,
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
/// A proof for a tree of the given size containing the audit path from a leaf to the root.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Proof {
    /// A sequence of 32 byte hashes used to reconstruct a Merkle Tree Hash.
    #[prost(bytes = "vec", tag = "1")]
    pub audit_path: ::prost::alloc::vec::Vec<u8>,
    /// The index of the leaf this proof applies to.
    #[prost(uint64, tag = "2")]
    pub leaf_index: u64,
    /// The total size of the tree this proof was derived from.
    #[prost(uint64, tag = "3")]
    pub tree_size: u64,
}
/// `RollupTransactions` are a sequence of opaque bytes together with a 32 byte
/// identifier of that rollup.
///
/// The binary encoding is understood as an implementation detail of the
/// services sending and receiving the transactions.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupTransactions {
    /// The 32 bytes identifying a rollup. Usually the sha256 hash of a plain rollup name.
    #[prost(bytes = "vec", tag = "1")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    /// The serialized opaque bytes of the rollup transactions.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
/// `SequencerBlock` is constructed from a tendermint/cometbft block by
/// converting its opaque `data` bytes into sequencer specific types.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlock {
    /// The original CometBFT header that was the input to this sequencer block.
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The collection of rollup transactions that were included in this block.
    #[prost(message, repeated, tag = "2")]
    pub rollup_transactions: ::prost::alloc::vec::Vec<RollupTransactions>,
    /// The proof that the rollup transactions are included in the CometBFT block this
    /// sequencer block is derived form. This proof together with
    /// `Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    /// `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    /// rollup transactions.
    #[prost(message, optional, tag = "3")]
    pub rollup_transactions_proof: ::core::option::Option<Proof>,
    /// The proof that the rollup IDs listed in `rollup_transactions` are included
    /// in the CometBFT block this sequencer block is derived form.
    ///
    /// This proof is used to verify that the relayer that posts to celestia
    /// includes all rollup IDs and does not censor any.
    ///
    /// This proof together with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    /// `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    /// the rollup transactions.
    #[prost(message, optional, tag = "4")]
    pub rollup_ids_proof: ::core::option::Option<Proof>,
}
/// A collection of transactions belonging to a specific rollup that are submitted to celestia.
///
/// The transactions contained in the item belong to a rollup identified
/// by `rollup_id`, and were included in the sequencer block identified
/// by `sequencer_block_hash`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CelestiaRollupBlob {
    /// The hash of the sequencer block. Must be 32 bytes.
    #[prost(bytes = "vec", tag = "1")]
    pub sequencer_block_hash: ::prost::alloc::vec::Vec<u8>,
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencer.v1alpha1.RollupTransactions.rollup_id`
    #[prost(bytes = "vec", tag = "2")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    /// A list of opaque bytes that are serialized rollup transactions.
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    /// `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions_proof`.
    #[prost(message, optional, tag = "4")]
    pub proof: ::core::option::Option<Proof>,
}
/// The metadata of a sequencer block that is submitted to celestia.
///
/// It is created by splitting a `astria.sequencer.v1alpha.SequencerBlock` into a
/// `CelestiaSequencerBlob` (which can be thought of as a header), and a sequence ofj
/// `CelestiaRollupBlob`s.
///
/// The original sequencer block (and in turn CometBFT block) can be identified by the
/// block hash calculated from `header`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CelestiaSequencerBlob {
    /// The original CometBFT header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.header`.
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1alpha1.RollupTransactions.rollup_id` field
    /// and is extracted from `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions`.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub rollup_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.sequencer.v1alpha.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    #[prost(bytes = "vec", tag = "3")]
    pub rollup_transactions_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions_proof`.
    #[prost(message, optional, tag = "4")]
    pub rollup_transactions_proof: ::core::option::Option<Proof>,
    /// The proof that the rollup IDs are included in sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.rollup_ids_proof`.
    #[prost(message, optional, tag = "5")]
    pub rollup_ids_proof: ::core::option::Option<Proof>,
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
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(oneof = "action::Value", tags = "1, 2, 3, 4, 5, 6, 7, 8")]
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
        IbcAction(::penumbra_proto::core::component::ibc::v1::IbcRelay),
        #[prost(message, tag = "7")]
        Ics20Withdrawal(super::Ics20Withdrawal),
        #[prost(message, tag = "8")]
        IbcRelayerChangeAction(super::IbcRelayerChangeAction),
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
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "4")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
/// `SequenceAction` represents a transaction destined for another
/// chain, ordered by the sequencer.
///
/// It contains the rollup ID of the destination chain, and the
/// opaque transaction data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequenceAction {
    #[prost(bytes = "vec", tag = "1")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ics20Withdrawal {
    /// first two fields are a transparent value consisting of an amount and a denom.
    #[prost(message, optional, tag = "1")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcHeight {
    #[prost(uint64, tag = "1")]
    pub revision_number: u64,
    #[prost(uint64, tag = "2")]
    pub revision_height: u64,
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
        #[prost(bytes, tag = "1")]
        Addition(::prost::alloc::vec::Vec<u8>),
        #[prost(bytes, tag = "2")]
        Removal(::prost::alloc::vec::Vec<u8>),
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
