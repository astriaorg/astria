#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBundleStreamRequest {}
impl ::prost::Name for GetBundleStreamRequest {
    const NAME: &'static str = "GetBundleStreamRequest";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
/// Information for the bundle submitter to know how to submit the bundle.
/// The fee and base_sequencer_block_hash are not necessarily strictly necessary
/// it allows for the case where the server doesn't always send the highest fee
/// bundles after the previous but could just stream any confirmed bundles.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Bundle {
    /// The fee that can be expected to be received for submitting this bundle.
    /// This allows the bundle producer to stream any confirmed bundles they would be ok
    /// with submitting. Used to avoid race conditions in received bundle packets. Could
    /// also be used by a bundle submitter to allow multiple entities to submit bundles.
    #[prost(uint64, tag = "1")]
    pub fee: u64,
    /// The byte list of transactions to be included.
    #[prost(bytes = "bytes", repeated, tag = "2")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
    /// The base_sequencer_block_hash is the hash from the base block this bundle
    /// is based on. This is used to verify that the bundle is based on the correct
    /// Sequencer block.
    #[prost(bytes = "bytes", tag = "3")]
    pub base_sequencer_block_hash: ::prost::bytes::Bytes,
    /// The hash of previous rollup block, on top of which the bundle will be executed as ToB.
    #[prost(bytes = "bytes", tag = "4")]
    pub prev_rollup_block_hash: ::prost::bytes::Bytes,
}
impl ::prost::Name for Bundle {
    const NAME: &'static str = "Bundle";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBundleStreamResponse {
    #[prost(message, optional, tag = "1")]
    pub bundle: ::core::option::Option<Bundle>,
}
impl ::prost::Name for GetBundleStreamResponse {
    const NAME: &'static str = "GetBundleStreamResponse";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod bundle_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct BundleServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl BundleServiceClient<tonic::transport::Channel> {
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
    impl<T> BundleServiceClient<T>
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
        ) -> BundleServiceClient<InterceptedService<T, F>>
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
            BundleServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// A bundle submitter requests bundles given a new optimistic Sequencer block,
        /// and receives a stream of potential bundles for submission, until either a timeout
        /// or the connection is closed by the client.
        pub async fn get_bundle_stream(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBundleStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::GetBundleStreamResponse>>,
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
                "/astria.bundle.v1alpha1.BundleService/GetBundleStream",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.bundle.v1alpha1.BundleService",
                        "GetBundleStream",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod bundle_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with BundleServiceServer.
    #[async_trait]
    pub trait BundleService: Send + Sync + 'static {
        /// Server streaming response type for the GetBundleStream method.
        type GetBundleStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<super::GetBundleStreamResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// A bundle submitter requests bundles given a new optimistic Sequencer block,
        /// and receives a stream of potential bundles for submission, until either a timeout
        /// or the connection is closed by the client.
        async fn get_bundle_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetBundleStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::GetBundleStreamStream>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct BundleServiceServer<T: BundleService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: BundleService> BundleServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for BundleServiceServer<T>
    where
        T: BundleService,
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
                "/astria.bundle.v1alpha1.BundleService/GetBundleStream" => {
                    #[allow(non_camel_case_types)]
                    struct GetBundleStreamSvc<T: BundleService>(pub Arc<T>);
                    impl<
                        T: BundleService,
                    > tonic::server::ServerStreamingService<
                        super::GetBundleStreamRequest,
                    > for GetBundleStreamSvc<T> {
                        type Response = super::GetBundleStreamResponse;
                        type ResponseStream = T::GetBundleStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBundleStreamRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as BundleService>::get_bundle_stream(inner, request)
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
                        let method = GetBundleStreamSvc(inner);
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
    impl<T: BundleService> Clone for BundleServiceServer<T> {
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
    impl<T: BundleService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: BundleService> tonic::server::NamedService for BundleServiceServer<T> {
        const NAME: &'static str = "astria.bundle.v1alpha1.BundleService";
    }
}
/// The "BaseBlock" is the information needed to simulate bundles on top of
/// a Sequencer block which may not have been committed yet.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BaseBlock {
    /// This is the block hash for the proposed block.
    #[prost(bytes = "bytes", tag = "1")]
    pub sequencer_block_hash: ::prost::bytes::Bytes,
    /// List of transactions to include in the new block.
    #[prost(message, repeated, tag = "2")]
    pub transactions: ::prost::alloc::vec::Vec<
        super::super::sequencerblock::v1::RollupData,
    >,
    /// Timestamp to be used for new block.
    #[prost(message, optional, tag = "3")]
    pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
}
impl ::prost::Name for BaseBlock {
    const NAME: &'static str = "BaseBlock";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteOptimisticBlockStreamRequest {
    #[prost(message, optional, tag = "1")]
    pub base_block: ::core::option::Option<BaseBlock>,
}
impl ::prost::Name for ExecuteOptimisticBlockStreamRequest {
    const NAME: &'static str = "ExecuteOptimisticBlockStreamRequest";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteOptimisticBlockStreamResponse {
    /// Metadata identifying the block resulting from executing a block. Includes number, hash,
    /// parent hash and timestamp.
    #[prost(message, optional, tag = "1")]
    pub block: ::core::option::Option<super::super::execution::v1::Block>,
    /// The base_sequencer_block_hash is the hash from the base sequencer block this block
    /// is based on. This is used to associate an optimistic execution result with the hash
    /// received once a sequencer block is committed.
    #[prost(bytes = "bytes", tag = "2")]
    pub base_sequencer_block_hash: ::prost::bytes::Bytes,
}
impl ::prost::Name for ExecuteOptimisticBlockStreamResponse {
    const NAME: &'static str = "ExecuteOptimisticBlockStreamResponse";
    const PACKAGE: &'static str = "astria.bundle.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.bundle.v1alpha1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod optimistic_execution_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct OptimisticExecutionServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl OptimisticExecutionServiceClient<tonic::transport::Channel> {
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
    impl<T> OptimisticExecutionServiceClient<T>
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
        ) -> OptimisticExecutionServiceClient<InterceptedService<T, F>>
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
            OptimisticExecutionServiceClient::new(
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
        /// Stream blocks from the Auctioneer to Geth for optimistic execution. Geth will stream back
        /// metadata from the executed blocks.
        pub async fn execute_optimistic_block_stream(
            &mut self,
            request: impl tonic::IntoStreamingRequest<
                Message = super::ExecuteOptimisticBlockStreamRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<
                tonic::codec::Streaming<super::ExecuteOptimisticBlockStreamResponse>,
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
                "/astria.bundle.v1alpha1.OptimisticExecutionService/ExecuteOptimisticBlockStream",
            );
            let mut req = request.into_streaming_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.bundle.v1alpha1.OptimisticExecutionService",
                        "ExecuteOptimisticBlockStream",
                    ),
                );
            self.inner.streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod optimistic_execution_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with OptimisticExecutionServiceServer.
    #[async_trait]
    pub trait OptimisticExecutionService: Send + Sync + 'static {
        /// Server streaming response type for the ExecuteOptimisticBlockStream method.
        type ExecuteOptimisticBlockStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<
                    super::ExecuteOptimisticBlockStreamResponse,
                    tonic::Status,
                >,
            >
            + Send
            + 'static;
        /// Stream blocks from the Auctioneer to Geth for optimistic execution. Geth will stream back
        /// metadata from the executed blocks.
        async fn execute_optimistic_block_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<
                tonic::Streaming<super::ExecuteOptimisticBlockStreamRequest>,
            >,
        ) -> std::result::Result<
            tonic::Response<Self::ExecuteOptimisticBlockStreamStream>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct OptimisticExecutionServiceServer<T: OptimisticExecutionService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: OptimisticExecutionService> OptimisticExecutionServiceServer<T> {
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
    for OptimisticExecutionServiceServer<T>
    where
        T: OptimisticExecutionService,
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
                "/astria.bundle.v1alpha1.OptimisticExecutionService/ExecuteOptimisticBlockStream" => {
                    #[allow(non_camel_case_types)]
                    struct ExecuteOptimisticBlockStreamSvc<
                        T: OptimisticExecutionService,
                    >(
                        pub Arc<T>,
                    );
                    impl<
                        T: OptimisticExecutionService,
                    > tonic::server::StreamingService<
                        super::ExecuteOptimisticBlockStreamRequest,
                    > for ExecuteOptimisticBlockStreamSvc<T> {
                        type Response = super::ExecuteOptimisticBlockStreamResponse;
                        type ResponseStream = T::ExecuteOptimisticBlockStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                tonic::Streaming<super::ExecuteOptimisticBlockStreamRequest>,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as OptimisticExecutionService>::execute_optimistic_block_stream(
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
                        let method = ExecuteOptimisticBlockStreamSvc(inner);
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
                        let res = grpc.streaming(method, req).await;
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
    impl<T: OptimisticExecutionService> Clone for OptimisticExecutionServiceServer<T> {
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
    impl<T: OptimisticExecutionService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: OptimisticExecutionService> tonic::server::NamedService
    for OptimisticExecutionServiceServer<T> {
        const NAME: &'static str = "astria.bundle.v1alpha1.OptimisticExecutionService";
    }
}
