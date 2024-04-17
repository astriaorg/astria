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
impl ::prost::Name for Proof {
    const NAME: &'static str = "Proof";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
    /// The serialized bytes of the rollup data.
    /// Each entry is a protobuf-encoded `RollupData` message.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    /// `astria.SequencerBlock.rollup_transactions_proof`.
    #[prost(message, optional, tag = "3")]
    pub proof: ::core::option::Option<Proof>,
}
impl ::prost::Name for RollupTransactions {
    const NAME: &'static str = "RollupTransactions";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// `SequencerBlock` is constructed from a tendermint/cometbft block by
/// converting its opaque `data` bytes into sequencer specific types.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlock {
    /// / the block header, which contains the cometbft header and additional sequencer-specific
    /// / commitments.
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<SequencerBlockHeader>,
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
impl ::prost::Name for SequencerBlock {
    const NAME: &'static str = "SequencerBlock";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlockHeader {
    /// The original CometBFT header that was the input to this sequencer block.
    #[prost(message, optional, tag = "1")]
    pub cometbft_header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The 32-byte merkle root of all the rollup transactions in the block,
    /// Corresponds to `MHT(astria.SequencerBlock.rollup_transactions)`,
    #[prost(bytes = "vec", tag = "2")]
    pub rollup_transactions_root: ::prost::alloc::vec::Vec<u8>,
    /// The 32-byte merkle root of all the rollup IDs in the block.
    #[prost(bytes = "vec", tag = "3")]
    pub rollup_ids_root: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SequencerBlockHeader {
    const NAME: &'static str = "SequencerBlockHeader";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// `Deposit` represents a deposit from the sequencer
/// to a rollup.
///
/// A `Deposit` is similar to an emitted event, in that the sequencer application detects
/// transfers to bridge accounts and the corresponding rollup ID and includes a `Deposit`
/// corresponding to that within the respective rollup's data.
///
/// A `Deposit` notifies a rollup that funds were locked to some account on the sequencer,
/// however it's up to the rollup what to do with that info.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Deposit {
    /// the address the funds were locked in on the sequencer.
    /// this is required as initializing an account as a bridge account
    /// is permissionless, so the rollup consensus needs to know and enshrine
    /// which accounts it accepts as valid bridge accounts.
    #[prost(bytes = "vec", tag = "1")]
    pub bridge_address: ::prost::alloc::vec::Vec<u8>,
    /// the rollup_id which the funds are being deposited to
    #[prost(bytes = "vec", tag = "2")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
    #[prost(bytes = "vec", tag = "4")]
    pub asset_id: ::prost::alloc::vec::Vec<u8>,
    /// the address on the destination chain which
    /// will receive the bridged funds
    #[prost(string, tag = "5")]
    pub destination_chain_address: ::prost::alloc::string::String,
}
impl ::prost::Name for Deposit {
    const NAME: &'static str = "Deposit";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// `FilteredSequencerBlock` is similar to `SequencerBlock` but with a subset
/// of the rollup transactions.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FilteredSequencerBlock {
    /// The original CometBFT header that was the input to this sequencer block.
    #[prost(message, optional, tag = "1")]
    pub cometbft_header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// A subset of rollup transactions that were included in this block.
    #[prost(message, repeated, tag = "2")]
    pub rollup_transactions: ::prost::alloc::vec::Vec<RollupTransactions>,
    /// The Merkle Tree Hash of all the rollup transactions in the block (not just the
    /// subset included). Corresponds to `MHT(astria.SequencerBlock.rollup_transactions)`,
    /// the Merkle Tree Hash derived from the rollup transactions.
    /// Always 32 bytes.
    #[prost(bytes = "vec", tag = "3")]
    pub rollup_transactions_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the rollup transactions are included in the CometBFT block this
    /// sequencer block is derived form. This proof together with
    /// `rollup_transactions_root = Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    /// `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    /// rollup transactions.
    #[prost(message, optional, tag = "4")]
    pub rollup_transactions_proof: ::core::option::Option<Proof>,
    /// The rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1.RollupTransactions.rollup_id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    /// Note that these are all the rollup IDs in the sequencer block, not merely those in
    /// `rollup_transactions` field. This is necessary to prove that no rollup IDs were omitted.
    #[prost(bytes = "vec", repeated, tag = "5")]
    pub all_rollup_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that the `rollup_ids` are included
    /// in the CometBFT block this sequencer block is derived form.
    ///
    /// This proof is used to verify that the relayer that posts to celestia
    /// includes all rollup IDs and does not censor any.
    ///
    /// This proof together with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    /// `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    /// the rollup transactions.
    #[prost(message, optional, tag = "6")]
    pub rollup_ids_proof: ::core::option::Option<Proof>,
}
impl ::prost::Name for FilteredSequencerBlock {
    const NAME: &'static str = "FilteredSequencerBlock";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// A piece of data that is sent to a rollup execution node.
///
/// The data can be either sequenced data (originating from a `SequenceAction`
/// submitted by a user) or a `Deposit` originating from a `BridgeLockAction`.
///
/// The rollup node receives this type from conductor and must decode them accordingly.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupData {
    #[prost(oneof = "rollup_data::Value", tags = "1, 2")]
    pub value: ::core::option::Option<rollup_data::Value>,
}
/// Nested message and enum types in `RollupData`.
pub mod rollup_data {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(bytes, tag = "1")]
        SequencedData(::prost::alloc::vec::Vec<u8>),
        #[prost(message, tag = "2")]
        Deposit(super::Deposit),
    }
}
impl ::prost::Name for RollupData {
    const NAME: &'static str = "RollupData";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
impl ::prost::Name for AssetBalance {
    const NAME: &'static str = "AssetBalance";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for BalanceResponse {
    const NAME: &'static str = "BalanceResponse";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for NonceResponse {
    const NAME: &'static str = "NonceResponse";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for Denom {
    const NAME: &'static str = "Denom";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
    /// `astria.sequencer.v1.RollupTransactions.rollup_id`
    #[prost(bytes = "vec", tag = "2")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    /// A list of opaque bytes that are serialized rollup transactions.
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    /// `astria.sequencerblock.v1alpha1.SequencerBlock.rollup_transactions_proof`.
    #[prost(message, optional, tag = "4")]
    pub proof: ::core::option::Option<Proof>,
}
impl ::prost::Name for CelestiaRollupBlob {
    const NAME: &'static str = "CelestiaRollupBlob";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// The metadata of a sequencer block that is submitted to celestia.
///
/// It is created by splitting a `astria.SequencerBlock` into a
/// `CelestiaSequencerBlob` (which can be thought of as a header), and a sequence ofj
/// `CelestiaRollupBlob`s.
///
/// The original sequencer block (and in turn CometBFT block) can be identified by the
/// block hash calculated from `header`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CelestiaSequencerBlob {
    /// The original CometBFT header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.SequencerBlock.header`.
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<::tendermint_proto::types::Header>,
    /// The rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1.RollupTransactions.rollup_id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub rollup_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    #[prost(bytes = "vec", tag = "3")]
    pub rollup_transactions_root: ::prost::alloc::vec::Vec<u8>,
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.sequencerblock.v1alpha1.SequencerBlock.rollup_transactions_proof`.
    #[prost(message, optional, tag = "4")]
    pub rollup_transactions_proof: ::core::option::Option<Proof>,
    /// The proof that the rollup IDs are included in sequencer block.
    /// Corresponds to `astria.sequencerblock.v1alpha1.SequencerBlock.rollup_ids_proof`.
    #[prost(message, optional, tag = "5")]
    pub rollup_ids_proof: ::core::option::Option<Proof>,
}
impl ::prost::Name for CelestiaSequencerBlob {
    const NAME: &'static str = "CelestiaSequencerBlob";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSequencerBlockRequest {
    /// The height of the block to retrieve.
    #[prost(uint64, tag = "1")]
    pub height: u64,
}
impl ::prost::Name for GetSequencerBlockRequest {
    const NAME: &'static str = "GetSequencerBlockRequest";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetFilteredSequencerBlockRequest {
    /// The height of the block to retrieve.
    #[prost(uint64, tag = "1")]
    pub height: u64,
    /// The 32 bytes identifying a rollup. Usually the sha256 hash of a plain rollup name.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub rollup_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for GetFilteredSequencerBlockRequest {
    const NAME: &'static str = "GetFilteredSequencerBlockRequest";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod sequencer_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct SequencerServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SequencerServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SequencerServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SequencerServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SequencerServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Given a block height, returns the sequencer block at that height.
        pub async fn get_sequencer_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSequencerBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::SequencerBlock>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/astria.sequencer.v1.SequencerService/GetSequencerBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.sequencer.v1.SequencerService",
                        "GetSequencerBlock",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Given a block height and set of rollup ids, returns a SequencerBlock which
        /// is filtered to contain only the transactions that are relevant to the given rollup.
        pub async fn get_filtered_sequencer_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetFilteredSequencerBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FilteredSequencerBlock>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/astria.sequencer.v1.SequencerService/GetFilteredSequencerBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.sequencer.v1.SequencerService",
                        "GetFilteredSequencerBlock",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod sequencer_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SequencerServiceServer.
    #[async_trait]
    pub trait SequencerService: Send + Sync + 'static {
        /// Given a block height, returns the sequencer block at that height.
        async fn get_sequencer_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetSequencerBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::SequencerBlock>, tonic::Status>;
        /// Given a block height and set of rollup ids, returns a SequencerBlock which
        /// is filtered to contain only the transactions that are relevant to the given rollup.
        async fn get_filtered_sequencer_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetFilteredSequencerBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FilteredSequencerBlock>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct SequencerServiceServer<T: SequencerService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: SequencerService> SequencerServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SequencerServiceServer<T>
    where
        T: SequencerService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/astria.sequencer.v1.SequencerService/GetSequencerBlock" => {
                    #[allow(non_camel_case_types)]
                    struct GetSequencerBlockSvc<T: SequencerService>(pub Arc<T>);
                    impl<
                        T: SequencerService,
                    > tonic::server::UnaryService<super::GetSequencerBlockRequest>
                    for GetSequencerBlockSvc<T> {
                        type Response = super::SequencerBlock;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetSequencerBlockRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SequencerService>::get_sequencer_block(inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetSequencerBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/astria.sequencer.v1.SequencerService/GetFilteredSequencerBlock" => {
                    #[allow(non_camel_case_types)]
                    struct GetFilteredSequencerBlockSvc<T: SequencerService>(pub Arc<T>);
                    impl<
                        T: SequencerService,
                    > tonic::server::UnaryService<
                        super::GetFilteredSequencerBlockRequest,
                    > for GetFilteredSequencerBlockSvc<T> {
                        type Response = super::FilteredSequencerBlock;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetFilteredSequencerBlockRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SequencerService>::get_filtered_sequencer_block(
                                        inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetFilteredSequencerBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: SequencerService> Clone for SequencerServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: SequencerService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: SequencerService> tonic::server::NamedService for SequencerServiceServer<T> {
        const NAME: &'static str = "astria.sequencer.v1.SequencerService";
    }
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
impl ::prost::Name for SignedTransaction {
    const NAME: &'static str = "SignedTransaction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for UnsignedTransaction {
    const NAME: &'static str = "UnsignedTransaction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Action {
    #[prost(oneof = "action::Value", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11")]
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
        #[prost(message, tag = "9")]
        FeeAssetChangeAction(super::FeeAssetChangeAction),
        #[prost(message, tag = "10")]
        InitBridgeAccountAction(super::InitBridgeAccountAction),
        #[prost(message, tag = "11")]
        BridgeLockAction(super::BridgeLockAction),
    }
}
impl ::prost::Name for Action {
    const NAME: &'static str = "Action";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
impl ::prost::Name for TransferAction {
    const NAME: &'static str = "TransferAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    #[prost(bytes = "vec", tag = "1")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SequenceAction {
    const NAME: &'static str = "SequenceAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    #[prost(bytes = "vec", tag = "1")]
    pub new_address: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SudoAddressChangeAction {
    const NAME: &'static str = "SudoAddressChangeAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for MintAction {
    const NAME: &'static str = "MintAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
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
impl ::prost::Name for Ics20Withdrawal {
    const NAME: &'static str = "Ics20Withdrawal";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
        #[prost(bytes, tag = "1")]
        Addition(::prost::alloc::vec::Vec<u8>),
        #[prost(bytes, tag = "2")]
        Removal(::prost::alloc::vec::Vec<u8>),
    }
}
impl ::prost::Name for IbcRelayerChangeAction {
    const NAME: &'static str = "IbcRelayerChangeAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    #[prost(bytes = "vec", tag = "1")]
    pub rollup_id: ::prost::alloc::vec::Vec<u8>,
    /// the asset IDs accepted as an incoming transfer by the bridge account
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub asset_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// the asset used to pay the transaction fee
    #[prost(bytes = "vec", tag = "3")]
    pub fee_asset_id: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for InitBridgeAccountAction {
    const NAME: &'static str = "InitBridgeAccountAction";
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
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
    #[prost(bytes = "vec", tag = "1")]
    pub to: ::prost::alloc::vec::Vec<u8>,
    /// the amount to transfer
    #[prost(message, optional, tag = "2")]
    pub amount: ::core::option::Option<super::super::primitive::v1::Uint128>,
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
    const PACKAGE: &'static str = "astria.sequencer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.sequencer.v1.{}", Self::NAME)
    }
}
