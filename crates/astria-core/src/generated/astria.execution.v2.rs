/// SequencerInfo contains the information needed to map sequencer block height
/// to rollup block number for driving execution.
///
/// This information is used to determine which sequencer & celestia data to
/// use from the Astria & Celestia networks, as well as define shutdown/restart
/// behavior of the Conductor.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerInfo {
    /// The rollup_id is the unique identifier for the rollup chain.
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::primitive::v1::RollupId>,
    /// The first block height on the sequencer chain to use for rollup transactions.
    /// This is mapped to `rollup_first_block_number`.
    #[prost(uint32, tag = "2")]
    pub sequencer_first_block_height: u32,
    /// The first rollup block number to be executed. This is mapped to `sequencer_first_block_height`.
    /// The minimum first block number is 1.
    #[prost(uint64, tag = "3")]
    pub rollup_first_block_number: u64,
    /// The final rollup block number to execute before either re-fetching sequencer
    /// info (restarting) or shutting down (determined by `halt_at_rollup_stop_number`).
    /// If 0, no stop block will be set.
    #[prost(uint64, tag = "4")]
    pub rollup_stop_block_number: u64,
    /// The allowed variance in celestia for sequencer blocks to have been posted.
    #[prost(uint64, tag = "5")]
    pub celestia_block_variance: u64,
    /// The ID of the Astria Sequencer network to retrieve Sequencer blocks from.
    /// Conductor implementations should verify that the Sequencer network they are connected to
    /// have this chain ID (if fetching soft Sequencer blocks), and verify that the Sequencer metadata
    /// blobs retrieved from Celestia contain this chain ID (if extracting firm Sequencer blocks from
    /// Celestia blobs).
    #[prost(string, tag = "6")]
    pub sequencer_chain_id: ::prost::alloc::string::String,
    /// The ID of the Celestia network to retrieve blobs from.
    /// Conductor implementations should verify that the Celestia network they are connected to have
    /// this chain ID (if extracting firm Sequencer blocks from Celestia blobs).
    #[prost(string, tag = "7")]
    pub celestia_chain_id: ::prost::alloc::string::String,
    /// Requests that Conductor halt at `rollup_stop_block_number` instead of re-fetching
    /// the sequencer info and continuing execution. This is a no-op if `rollup_stop_block_number`
    /// is set to 0.
    #[prost(bool, tag = "8")]
    pub halt_at_rollup_stop_number: bool,
}
impl ::prost::Name for SequencerInfo {
    const NAME: &'static str = "SequencerInfo";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// The set of information which deterministic driver of block production
/// must know about a given rollup Block
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Block {
    /// The block number
    #[prost(uint32, tag = "1")]
    pub number: u32,
    /// The hash of the block
    #[prost(bytes = "bytes", tag = "2")]
    pub hash: ::prost::bytes::Bytes,
    /// The hash from the parent block
    #[prost(bytes = "bytes", tag = "3")]
    pub parent_block_hash: ::prost::bytes::Bytes,
    /// Timestamp on the block, standardized to google protobuf standard.
    #[prost(message, optional, tag = "4")]
    pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
}
impl ::prost::Name for Block {
    const NAME: &'static str = "Block";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Fields which are indexed for finding blocks on a blockchain.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockIdentifier {
    #[prost(oneof = "block_identifier::Identifier", tags = "1, 2")]
    pub identifier: ::core::option::Option<block_identifier::Identifier>,
}
/// Nested message and enum types in `BlockIdentifier`.
pub mod block_identifier {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Identifier {
        #[prost(uint32, tag = "1")]
        BlockNumber(u32),
        #[prost(bytes, tag = "2")]
        BlockHash(::prost::bytes::Bytes),
    }
}
impl ::prost::Name for BlockIdentifier {
    const NAME: &'static str = "BlockIdentifier";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Used to fetch the current `SequencerInfo` from the rollup.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSequencerInfoRequest {
    /// The commitment type that the sequencer info is being fetched for. If the commitment
    /// type is soft, the returned sequencer info should be based on the rollup's soft
    /// commitment height. If the commitment type is firm, the returned sequencer info
    /// should be based on the rollup's firm commitment height.
    #[prost(enumeration = "CommitmentType", tag = "1")]
    pub commitment_type: i32,
}
impl ::prost::Name for GetSequencerInfoRequest {
    const NAME: &'static str = "GetSequencerInfoRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Used in GetBlock to find a single block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockRequest {
    #[prost(message, optional, tag = "1")]
    pub identifier: ::core::option::Option<BlockIdentifier>,
}
impl ::prost::Name for GetBlockRequest {
    const NAME: &'static str = "GetBlockRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Used in BatchGetBlocks, will find all or none based on the list of
/// identifiers.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksRequest {
    #[prost(message, repeated, tag = "1")]
    pub identifiers: ::prost::alloc::vec::Vec<BlockIdentifier>,
}
impl ::prost::Name for BatchGetBlocksRequest {
    const NAME: &'static str = "BatchGetBlocksRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// The list of blocks in response to BatchGetBlocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetBlocksResponse {
    #[prost(message, repeated, tag = "1")]
    pub blocks: ::prost::alloc::vec::Vec<Block>,
}
impl ::prost::Name for BatchGetBlocksResponse {
    const NAME: &'static str = "BatchGetBlocksResponse";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// ExecuteBlockRequest contains all the information needed to create a new rollup
/// block.
///
/// This information comes from previous rollup blocks, as well as from sequencer
/// blocks.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteBlockRequest {
    /// The hash of previous block, which new block will be created on top of.
    #[prost(bytes = "bytes", tag = "1")]
    pub prev_block_hash: ::prost::bytes::Bytes,
    /// List of transactions to include in the new block.
    #[prost(message, repeated, tag = "2")]
    pub transactions: ::prost::alloc::vec::Vec<
        super::super::sequencerblock::v1::RollupData,
    >,
    /// Timestamp to be used for new block.
    #[prost(message, optional, tag = "3")]
    pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
}
impl ::prost::Name for ExecuteBlockRequest {
    const NAME: &'static str = "ExecuteBlockRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// The CommitmentState holds the block at each stage of sequencer commitment
/// level
///
/// A Valid CommitmentState:
/// - Block numbers are such that soft >= firm.
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the head of the canonical chain the firm block
///    must belong to.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitmentState {
    /// Soft commitment is the rollup block matching latest sequencer block.
    #[prost(message, optional, tag = "1")]
    pub soft: ::core::option::Option<Block>,
    /// Firm commitment is achieved when data has been seen in DA.
    #[prost(message, optional, tag = "2")]
    pub firm: ::core::option::Option<Block>,
    /// The lowest block number of celestia chain to be searched for rollup blocks
    /// given current state
    #[prost(uint64, tag = "3")]
    pub base_celestia_height: u64,
}
impl ::prost::Name for CommitmentState {
    const NAME: &'static str = "CommitmentState";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// There is only one CommitmentState object, so the request is empty.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommitmentStateRequest {}
impl ::prost::Name for GetCommitmentStateRequest {
    const NAME: &'static str = "GetCommitmentStateRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// The CommitmentState to set, must include complete state.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateCommitmentStateRequest {
    #[prost(message, optional, tag = "1")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
impl ::prost::Name for UpdateCommitmentStateRequest {
    const NAME: &'static str = "UpdateCommitmentStateRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Used in `GetSequencerInfoRequest` to obtain the corresponding sequencer info
/// for the rollup block number with the given commitment type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CommitmentType {
    Unspecified = 0,
    Soft = 1,
    Firm = 2,
}
impl CommitmentType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            CommitmentType::Unspecified => "COMMITMENT_TYPE_UNSPECIFIED",
            CommitmentType::Soft => "COMMITMENT_TYPE_SOFT",
            CommitmentType::Firm => "COMMITMENT_TYPE_FIRM",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "COMMITMENT_TYPE_UNSPECIFIED" => Some(Self::Unspecified),
            "COMMITMENT_TYPE_SOFT" => Some(Self::Soft),
            "COMMITMENT_TYPE_FIRM" => Some(Self::Firm),
            _ => None,
        }
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod execution_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// ExecutionService is used to drive deterministic production of blocks.
    ///
    /// The service can be implemented by any blockchain which wants to utilize the
    /// Astria Shared Sequencer, and will have block production driven via the Astria
    /// "Conductor".
    #[derive(Debug, Clone)]
    pub struct ExecutionServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ExecutionServiceClient<tonic::transport::Channel> {
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
    impl<T> ExecutionServiceClient<T>
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
        ) -> ExecutionServiceClient<InterceptedService<T, F>>
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
            ExecutionServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// GetSequencerInfo returns the necessary information for mapping sequencer block
        /// height to rollup block number.
        pub async fn get_sequencer_info(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSequencerInfoRequest>,
        ) -> std::result::Result<tonic::Response<super::SequencerInfo>, tonic::Status> {
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
                "/astria.execution.v2.ExecutionService/GetSequencerInfo",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "GetSequencerInfo",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// GetBlock will return a block given an identifier.
        pub async fn get_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::Block>, tonic::Status> {
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
                "/astria.execution.v2.ExecutionService/GetBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("astria.execution.v2.ExecutionService", "GetBlock"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// BatchGetBlocks will return an array of Blocks given an array of block
        /// identifiers.
        pub async fn batch_get_blocks(
            &mut self,
            request: impl tonic::IntoRequest<super::BatchGetBlocksRequest>,
        ) -> std::result::Result<
            tonic::Response<super::BatchGetBlocksResponse>,
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
                "/astria.execution.v2.ExecutionService/BatchGetBlocks",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "BatchGetBlocks",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// ExecuteBlock is called to deterministically derive a rollup block from
        /// filtered sequencer block information.
        pub async fn execute_block(
            &mut self,
            request: impl tonic::IntoRequest<super::ExecuteBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::Block>, tonic::Status> {
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
                "/astria.execution.v2.ExecutionService/ExecuteBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "ExecuteBlock",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// GetCommitmentState fetches the current CommitmentState of the chain.
        pub async fn get_commitment_state(
            &mut self,
            request: impl tonic::IntoRequest<super::GetCommitmentStateRequest>,
        ) -> std::result::Result<
            tonic::Response<super::CommitmentState>,
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
                "/astria.execution.v2.ExecutionService/GetCommitmentState",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "GetCommitmentState",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// UpdateCommitmentState replaces the whole CommitmentState with a new
        /// CommitmentState.
        pub async fn update_commitment_state(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateCommitmentStateRequest>,
        ) -> std::result::Result<
            tonic::Response<super::CommitmentState>,
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
                "/astria.execution.v2.ExecutionService/UpdateCommitmentState",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "UpdateCommitmentState",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod execution_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ExecutionServiceServer.
    #[async_trait]
    pub trait ExecutionService: Send + Sync + 'static {
        /// GetSequencerInfo returns the necessary information for mapping sequencer block
        /// height to rollup block number.
        async fn get_sequencer_info(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetSequencerInfoRequest>,
        ) -> std::result::Result<tonic::Response<super::SequencerInfo>, tonic::Status>;
        /// GetBlock will return a block given an identifier.
        async fn get_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::Block>, tonic::Status>;
        /// BatchGetBlocks will return an array of Blocks given an array of block
        /// identifiers.
        async fn batch_get_blocks(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::BatchGetBlocksRequest>,
        ) -> std::result::Result<
            tonic::Response<super::BatchGetBlocksResponse>,
            tonic::Status,
        >;
        /// ExecuteBlock is called to deterministically derive a rollup block from
        /// filtered sequencer block information.
        async fn execute_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::ExecuteBlockRequest>,
        ) -> std::result::Result<tonic::Response<super::Block>, tonic::Status>;
        /// GetCommitmentState fetches the current CommitmentState of the chain.
        async fn get_commitment_state(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetCommitmentStateRequest>,
        ) -> std::result::Result<tonic::Response<super::CommitmentState>, tonic::Status>;
        /// UpdateCommitmentState replaces the whole CommitmentState with a new
        /// CommitmentState.
        async fn update_commitment_state(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::UpdateCommitmentStateRequest>,
        ) -> std::result::Result<tonic::Response<super::CommitmentState>, tonic::Status>;
    }
    /// ExecutionService is used to drive deterministic production of blocks.
    ///
    /// The service can be implemented by any blockchain which wants to utilize the
    /// Astria Shared Sequencer, and will have block production driven via the Astria
    /// "Conductor".
    #[derive(Debug)]
    pub struct ExecutionServiceServer<T: ExecutionService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ExecutionService> ExecutionServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ExecutionServiceServer<T>
    where
        T: ExecutionService,
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
                "/astria.execution.v2.ExecutionService/GetSequencerInfo" => {
                    #[allow(non_camel_case_types)]
                    struct GetSequencerInfoSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::GetSequencerInfoRequest>
                    for GetSequencerInfoSvc<T> {
                        type Response = super::SequencerInfo;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetSequencerInfoRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::get_sequencer_info(inner, request)
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
                        let method = GetSequencerInfoSvc(inner);
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
                "/astria.execution.v2.ExecutionService/GetBlock" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::GetBlockRequest>
                    for GetBlockSvc<T> {
                        type Response = super::Block;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::get_block(inner, request).await
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
                        let method = GetBlockSvc(inner);
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
                "/astria.execution.v2.ExecutionService/BatchGetBlocks" => {
                    #[allow(non_camel_case_types)]
                    struct BatchGetBlocksSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::BatchGetBlocksRequest>
                    for BatchGetBlocksSvc<T> {
                        type Response = super::BatchGetBlocksResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BatchGetBlocksRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::batch_get_blocks(inner, request)
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
                        let method = BatchGetBlocksSvc(inner);
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
                "/astria.execution.v2.ExecutionService/ExecuteBlock" => {
                    #[allow(non_camel_case_types)]
                    struct ExecuteBlockSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::ExecuteBlockRequest>
                    for ExecuteBlockSvc<T> {
                        type Response = super::Block;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ExecuteBlockRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::execute_block(inner, request).await
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
                        let method = ExecuteBlockSvc(inner);
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
                "/astria.execution.v2.ExecutionService/GetCommitmentState" => {
                    #[allow(non_camel_case_types)]
                    struct GetCommitmentStateSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::GetCommitmentStateRequest>
                    for GetCommitmentStateSvc<T> {
                        type Response = super::CommitmentState;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetCommitmentStateRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::get_commitment_state(
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
                        let method = GetCommitmentStateSvc(inner);
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
                "/astria.execution.v2.ExecutionService/UpdateCommitmentState" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateCommitmentStateSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::UpdateCommitmentStateRequest>
                    for UpdateCommitmentStateSvc<T> {
                        type Response = super::CommitmentState;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdateCommitmentStateRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::update_commitment_state(
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
                        let method = UpdateCommitmentStateSvc(inner);
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
    impl<T: ExecutionService> Clone for ExecutionServiceServer<T> {
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
    impl<T: ExecutionService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ExecutionService> tonic::server::NamedService for ExecutionServiceServer<T> {
        const NAME: &'static str = "astria.execution.v2.ExecutionService";
    }
}
