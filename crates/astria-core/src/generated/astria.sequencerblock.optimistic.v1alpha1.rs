#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockCommitmentStreamRequest {}
impl ::prost::Name for GetBlockCommitmentStreamRequest {
    const NAME: &'static str = "GetBlockCommitmentStreamRequest";
    const PACKAGE: &'static str = "astria.sequencerblock.optimistic.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.sequencerblock.optimistic.v1alpha1.{}", Self::NAME
        )
    }
}
/// Identifying metadata for blocks that have been successfully committed in the Sequencer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SequencerBlockCommit {
    /// Height of the sequencer block that was committed.
    #[prost(uint64, tag = "1")]
    pub height: u64,
    /// Hash of the sequencer block that was committed.
    #[prost(bytes = "bytes", tag = "2")]
    pub block_hash: ::prost::bytes::Bytes,
}
impl ::prost::Name for SequencerBlockCommit {
    const NAME: &'static str = "SequencerBlockCommit";
    const PACKAGE: &'static str = "astria.sequencerblock.optimistic.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.sequencerblock.optimistic.v1alpha1.{}", Self::NAME
        )
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockCommitmentStreamResponse {
    #[prost(message, optional, tag = "1")]
    pub commitment: ::core::option::Option<SequencerBlockCommit>,
}
impl ::prost::Name for GetBlockCommitmentStreamResponse {
    const NAME: &'static str = "GetBlockCommitmentStreamResponse";
    const PACKAGE: &'static str = "astria.sequencerblock.optimistic.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.sequencerblock.optimistic.v1alpha1.{}", Self::NAME
        )
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetOptimisticBlockStreamRequest {
    /// The rollup id for which the Sequencer block is being streamed.
    #[prost(message, optional, tag = "1")]
    pub rollup_id: ::core::option::Option<super::super::super::primitive::v1::RollupId>,
}
impl ::prost::Name for GetOptimisticBlockStreamRequest {
    const NAME: &'static str = "GetOptimisticBlockStreamRequest";
    const PACKAGE: &'static str = "astria.sequencerblock.optimistic.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.sequencerblock.optimistic.v1alpha1.{}", Self::NAME
        )
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetOptimisticBlockStreamResponse {
    /// The optimistic Sequencer block that is being streamed, filtered for the provided rollup id.
    #[prost(message, optional, tag = "1")]
    pub block: ::core::option::Option<super::super::v1::FilteredSequencerBlock>,
}
impl ::prost::Name for GetOptimisticBlockStreamResponse {
    const NAME: &'static str = "GetOptimisticBlockStreamResponse";
    const PACKAGE: &'static str = "astria.sequencerblock.optimistic.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!(
            "astria.sequencerblock.optimistic.v1alpha1.{}", Self::NAME
        )
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod optimistic_block_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The Sequencer will serve this to the aucitoneer
    #[derive(Debug, Clone)]
    pub struct OptimisticBlockServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl OptimisticBlockServiceClient<tonic::transport::Channel> {
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
    impl<T> OptimisticBlockServiceClient<T>
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
        ) -> OptimisticBlockServiceClient<InterceptedService<T, F>>
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
            OptimisticBlockServiceClient::new(
                InterceptedService::new(inner, interceptor),
            )
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
        /// The Sequencer will stream the optimistic Sequencer block (filtered for the provided
        /// rollup id) to the Auctioneer.
        pub async fn get_optimistic_block_stream(
            &mut self,
            request: impl tonic::IntoRequest<super::GetOptimisticBlockStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<
                tonic::codec::Streaming<super::GetOptimisticBlockStreamResponse>,
            >,
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
                "/astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService/GetOptimisticBlockStream",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService",
                        "GetOptimisticBlockStream",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
        /// The Sequencer will stream the block commits to the Auctioneer.
        pub async fn get_block_commitment_stream(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockCommitmentStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<
                tonic::codec::Streaming<super::GetBlockCommitmentStreamResponse>,
            >,
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
                "/astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService/GetBlockCommitmentStream",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService",
                        "GetBlockCommitmentStream",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod optimistic_block_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with OptimisticBlockServiceServer.
    #[async_trait]
    pub trait OptimisticBlockService: Send + Sync + 'static {
        /// Server streaming response type for the GetOptimisticBlockStream method.
        type GetOptimisticBlockStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<
                    super::GetOptimisticBlockStreamResponse,
                    tonic::Status,
                >,
            >
            + Send
            + 'static;
        /// The Sequencer will stream the optimistic Sequencer block (filtered for the provided
        /// rollup id) to the Auctioneer.
        async fn get_optimistic_block_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetOptimisticBlockStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::GetOptimisticBlockStreamStream>,
            tonic::Status,
        >;
        /// Server streaming response type for the GetBlockCommitmentStream method.
        type GetBlockCommitmentStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<
                    super::GetBlockCommitmentStreamResponse,
                    tonic::Status,
                >,
            >
            + Send
            + 'static;
        /// The Sequencer will stream the block commits to the Auctioneer.
        async fn get_block_commitment_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetBlockCommitmentStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::GetBlockCommitmentStreamStream>,
            tonic::Status,
        >;
    }
    /// The Sequencer will serve this to the aucitoneer
    #[derive(Debug)]
    pub struct OptimisticBlockServiceServer<T: OptimisticBlockService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: OptimisticBlockService> OptimisticBlockServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>>
    for OptimisticBlockServiceServer<T>
    where
        T: OptimisticBlockService,
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
                "/astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService/GetOptimisticBlockStream" => {
                    #[allow(non_camel_case_types)]
                    struct GetOptimisticBlockStreamSvc<T: OptimisticBlockService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: OptimisticBlockService,
                    > tonic::server::ServerStreamingService<
                        super::GetOptimisticBlockStreamRequest,
                    > for GetOptimisticBlockStreamSvc<T> {
                        type Response = super::GetOptimisticBlockStreamResponse;
                        type ResponseStream = T::GetOptimisticBlockStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetOptimisticBlockStreamRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as OptimisticBlockService>::get_optimistic_block_stream(
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
                        let method = GetOptimisticBlockStreamSvc(inner);
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
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService/GetBlockCommitmentStream" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockCommitmentStreamSvc<T: OptimisticBlockService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: OptimisticBlockService,
                    > tonic::server::ServerStreamingService<
                        super::GetBlockCommitmentStreamRequest,
                    > for GetBlockCommitmentStreamSvc<T> {
                        type Response = super::GetBlockCommitmentStreamResponse;
                        type ResponseStream = T::GetBlockCommitmentStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetBlockCommitmentStreamRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as OptimisticBlockService>::get_block_commitment_stream(
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
                        let method = GetBlockCommitmentStreamSvc(inner);
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
                        let res = grpc.server_streaming(method, req).await;
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
    impl<T: OptimisticBlockService> Clone for OptimisticBlockServiceServer<T> {
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
    impl<T: OptimisticBlockService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: OptimisticBlockService> tonic::server::NamedService
    for OptimisticBlockServiceServer<T> {
        const NAME: &'static str = "astria.sequencerblock.optimistic.v1alpha1.OptimisticBlockService";
    }
}
