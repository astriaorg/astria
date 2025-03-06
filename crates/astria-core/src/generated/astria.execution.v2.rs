/// The set of information which deterministic driver of block production
/// must know about a given rollup Block
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecutedBlockMetadata {
    /// The block number
    #[prost(uint64, tag = "1")]
    pub number: u64,
    /// The hash of the block, formatted in the execution node's preferred encoding.
    #[prost(string, tag = "2")]
    pub hash: ::prost::alloc::string::String,
    /// The hash of this block's parent block, formatted in the execution node's preferred
    /// encoding.
    #[prost(string, tag = "3")]
    pub parent_hash: ::prost::alloc::string::String,
    /// Timestamp of the block, taken from the sequencer block that this rollup block
    /// was constructed from.
    #[prost(message, optional, tag = "4")]
    pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
    /// The hash of the sequencer block from which this block was derived.
    ///
    /// Must be 32 byte base16 encoded string. It may be prefixed with `0x`.
    ///
    /// (Optional) This field will only be utilized if the execution node stores
    /// this data in blocks during `ExecuteBlock`.
    #[prost(string, tag = "5")]
    pub sequencer_block_hash: ::prost::alloc::string::String,
}
impl ::prost::Name for ExecutedBlockMetadata {
    const NAME: &'static str = "ExecutedBlockMetadata";
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
    /// Soft committed block metadata derived directly from an Astria sequencer block.
    #[prost(message, optional, tag = "1")]
    pub soft_executed_block_metadata: ::core::option::Option<ExecutedBlockMetadata>,
    /// Firm committed block metadata derived from a Sequencer block that has been
    /// written to the data availability layer (Celestia).
    #[prost(message, optional, tag = "2")]
    pub firm_executed_block_metadata: ::core::option::Option<ExecutedBlockMetadata>,
    /// The lowest Celestia height that will be searched for the next firm block.
    /// This information is stored as part of `CommitmentState` so that it will be
    /// routinely updated as new firm blocks are received, and so that the execution
    /// client will not need to search from Celestia genesis.
    #[prost(uint64, tag = "3")]
    pub lowest_celestia_search_height: u64,
}
impl ::prost::Name for CommitmentState {
    const NAME: &'static str = "CommitmentState";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// CreateExecutionSessionRequest is used to create a new execution session on the
/// rollup.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateExecutionSessionRequest {}
impl ::prost::Name for CreateExecutionSessionRequest {
    const NAME: &'static str = "CreateExecutionSessionRequest";
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
    /// The session within which the block is intended to be executed.
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The hash of previous block, which this new block will be created on top of,
    /// formatted in the execution node's preferred encoding.
    #[prost(string, tag = "2")]
    pub parent_hash: ::prost::alloc::string::String,
    /// List of transactions to include in the new block.
    #[prost(message, repeated, tag = "3")]
    pub transactions: ::prost::alloc::vec::Vec<
        super::super::sequencerblock::v1::RollupData,
    >,
    /// Timestamp to be used for new block.
    #[prost(message, optional, tag = "4")]
    pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
    /// The hash of the sequencer block from which the transactions and timestamp
    /// are derived.
    ///
    /// Must be a 32 byte base16 encoded string. It may be prefixed with `0x`.
    ///
    /// Utilizing this field is optional for the execution node.
    #[prost(string, tag = "5")]
    pub sequencer_block_hash: ::prost::alloc::string::String,
}
impl ::prost::Name for ExecuteBlockRequest {
    const NAME: &'static str = "ExecuteBlockRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// ExecuteBlockResponse is the response type for the ExecuteBlock RPC. It contains
/// the metadata of the block which was executed against the rollup.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteBlockResponse {
    #[prost(message, optional, tag = "1")]
    pub executed_block_metadata: ::core::option::Option<ExecutedBlockMetadata>,
}
impl ::prost::Name for ExecuteBlockResponse {
    const NAME: &'static str = "ExecuteBlockResponse";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Identifiers to select an executed block by.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecutedBlockIdentifier {
    #[prost(oneof = "executed_block_identifier::Identifier", tags = "1, 2")]
    pub identifier: ::core::option::Option<executed_block_identifier::Identifier>,
}
/// Nested message and enum types in `ExecutedBlockIdentifier`.
pub mod executed_block_identifier {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Identifier {
        /// Identifier by block number, corresponding to `ExecutedBlockMetadata.number`.
        #[prost(uint64, tag = "1")]
        Number(u64),
        /// Identifier by block hash, corresponding to `ExecutedBlockMetadata.hash`.
        #[prost(string, tag = "2")]
        Hash(::prost::alloc::string::String),
    }
}
impl ::prost::Name for ExecutedBlockIdentifier {
    const NAME: &'static str = "ExecutedBlockIdentifier";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// ExecutionSessionParameters contains the information needed to map sequencer block height
/// to rollup block number for driving execution.
///
/// This information is used to determine which Astria sequencer and Celestia data
/// to use from the Astria & Celestia networks, as well as define the bounds of
/// block numbers to execute in the given session.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecutionSessionParameters {
    /// The rollup_id is the unique identifier for the rollup chain.
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::primitive::v1::RollupId>,
    /// The first rollup block number to be executed. This is mapped to `sequencer_first_block_height`.
    /// The minimum first block number is 1, since 0 represents the genesis block.
    /// Implementors should reject a value of 0.
    ///
    /// Servers implementing this API should reject execution of blocks below this
    /// value with an OUT_OF_RANGE error code.
    #[prost(uint64, tag = "2")]
    pub rollup_start_block_number: u64,
    /// The final rollup block number to execute as part of a session.
    ///
    /// If not set or set to 0, the execution session does not have an upper bound.
    ///
    /// Servers implementing this API should reject execution of blocks past this
    /// value with an OUT_OF_RANGE error code.
    #[prost(uint64, tag = "3")]
    pub rollup_end_block_number: u64,
    /// The ID of the Astria Sequencer network to retrieve Sequencer blocks from.
    /// Conductor implementations should verify that the Sequencer network they are
    /// connected to have this chain ID (if fetching soft Sequencer blocks), and verify
    /// that the Sequencer metadata blobs retrieved from Celestia contain this chain
    /// ID (if extracting firm Sequencer blocks from Celestia blobs).
    #[prost(string, tag = "4")]
    pub sequencer_chain_id: ::prost::alloc::string::String,
    /// The first block height on the sequencer chain to use for rollup transactions.
    /// This is mapped to `rollup_start_block_number`.
    #[prost(uint64, tag = "5")]
    pub sequencer_start_block_height: u64,
    /// The ID of the Celestia network to retrieve blobs from.
    /// Conductor implementations should verify that the Celestia network they are
    /// connected to have this chain ID (if extracting firm Sequencer blocks from
    /// Celestia blobs).
    #[prost(string, tag = "6")]
    pub celestia_chain_id: ::prost::alloc::string::String,
    /// The maximum number of Celestia blocks which can be read above
    /// `CommitmentState.lowest_celestia_search_height` in search of the next firm
    /// block.
    #[prost(uint64, tag = "7")]
    pub celestia_search_height_max_look_ahead: u64,
}
impl ::prost::Name for ExecutionSessionParameters {
    const NAME: &'static str = "ExecutionSessionParameters";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// ExecutionSession contains the information needed to drive the full execution
/// of a rollup chain in the rollup.
///
/// The execution session is only valid for the execution config params with
/// which it was created. Once all blocks within the session have been executed,
/// the execution client must request a new session. The session_id is used to
/// to track which session is being used.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecutionSession {
    /// An ID for the session.
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The configuration for the execution session.
    #[prost(message, optional, tag = "2")]
    pub execution_session_parameters: ::core::option::Option<ExecutionSessionParameters>,
    /// The commitment state for executing client to start from.
    #[prost(message, optional, tag = "3")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
impl ::prost::Name for ExecutionSession {
    const NAME: &'static str = "ExecutionSession";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// Used in GetExecutedBlockMetadata to find a single block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetExecutedBlockMetadataRequest {
    #[prost(message, optional, tag = "1")]
    pub identifier: ::core::option::Option<ExecutedBlockIdentifier>,
}
impl ::prost::Name for GetExecutedBlockMetadataRequest {
    const NAME: &'static str = "GetExecutedBlockMetadataRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
    }
}
/// The CommitmentState to set, must include complete state.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateCommitmentStateRequest {
    /// The session which the commitment state is being updated within.
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    /// The new commitment state to set.
    #[prost(message, optional, tag = "2")]
    pub commitment_state: ::core::option::Option<CommitmentState>,
}
impl ::prost::Name for UpdateCommitmentStateRequest {
    const NAME: &'static str = "UpdateCommitmentStateRequest";
    const PACKAGE: &'static str = "astria.execution.v2";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.execution.v2.{}", Self::NAME)
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
        /// CreateExecutionSession returns the necessary information for mapping sequencer block
        /// height to rollup block number.
        pub async fn create_execution_session(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateExecutionSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecutionSession>,
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
                "/astria.execution.v2.ExecutionService/CreateExecutionSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "CreateExecutionSession",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// GetExecutedBlockMetadata will return a block given an identifier.
        pub async fn get_executed_block_metadata(
            &mut self,
            request: impl tonic::IntoRequest<super::GetExecutedBlockMetadataRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecutedBlockMetadata>,
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
                "/astria.execution.v2.ExecutionService/GetExecutedBlockMetadata",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.execution.v2.ExecutionService",
                        "GetExecutedBlockMetadata",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// ExecuteBlock is called to deterministically derive a rollup block from
        /// filtered sequencer block information.
        pub async fn execute_block(
            &mut self,
            request: impl tonic::IntoRequest<super::ExecuteBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecuteBlockResponse>,
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
        /// CreateExecutionSession returns the necessary information for mapping sequencer block
        /// height to rollup block number.
        async fn create_execution_session(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::CreateExecutionSessionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecutionSession>,
            tonic::Status,
        >;
        /// GetExecutedBlockMetadata will return a block given an identifier.
        async fn get_executed_block_metadata(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetExecutedBlockMetadataRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecutedBlockMetadata>,
            tonic::Status,
        >;
        /// ExecuteBlock is called to deterministically derive a rollup block from
        /// filtered sequencer block information.
        async fn execute_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::ExecuteBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ExecuteBlockResponse>,
            tonic::Status,
        >;
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
                "/astria.execution.v2.ExecutionService/CreateExecutionSession" => {
                    #[allow(non_camel_case_types)]
                    struct CreateExecutionSessionSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::CreateExecutionSessionRequest>
                    for CreateExecutionSessionSvc<T> {
                        type Response = super::ExecutionSession;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateExecutionSessionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::create_execution_session(
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
                        let method = CreateExecutionSessionSvc(inner);
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
                "/astria.execution.v2.ExecutionService/GetExecutedBlockMetadata" => {
                    #[allow(non_camel_case_types)]
                    struct GetExecutedBlockMetadataSvc<T: ExecutionService>(pub Arc<T>);
                    impl<
                        T: ExecutionService,
                    > tonic::server::UnaryService<super::GetExecutedBlockMetadataRequest>
                    for GetExecutedBlockMetadataSvc<T> {
                        type Response = super::ExecutedBlockMetadata;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetExecutedBlockMetadataRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ExecutionService>::get_executed_block_metadata(
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
                        let method = GetExecutedBlockMetadataSvc(inner);
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
                        type Response = super::ExecuteBlockResponse;
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
