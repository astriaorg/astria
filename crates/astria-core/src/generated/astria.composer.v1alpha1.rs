
use crate::generated::v1alpha1;

/// SubmitRollupTransactionRequest contains a rollup transaction to be submitted to the Shared Sequencer Network
/// via the Composer
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitRollupTransactionRequest {
    /// the unhashed rollup id
    #[prost(bytes = "bytes", tag = "1")]
    pub rollup_id: ::prost::bytes::Bytes,
    /// the raw data bytes of the rollup transaction
    #[prost(bytes = "bytes", tag = "2")]
    pub data: ::prost::bytes::Bytes,
}
impl ::prost::Name for SubmitRollupTransactionRequest {
    const NAME: &'static str = "SubmitRollupTransactionRequest";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
/// SubmitRollupTransactionResponse is a message that represents a response to a request to submit a rollup.
/// It's currently an empty response which can be evolved in the future to include more information
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitRollupTransactionResponse {}
impl ::prost::Name for SubmitRollupTransactionResponse {
    const NAME: &'static str = "SubmitRollupTransactionResponse";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendOptimisticBlockRequest {
    #[prost(bytes = "bytes", tag = "1")]
    pub block_hash: ::prost::bytes::Bytes,
    #[prost(message, repeated, tag = "2")]
    pub seq_action: ::prost::alloc::vec::Vec<
        v1alpha1::SequenceAction,
    >,
    #[prost(message, optional, tag = "3")]
    pub time: ::core::option::Option<::pbjson_types::Timestamp>,
}
impl ::prost::Name for SendOptimisticBlockRequest {
    const NAME: &'static str = "SendOptimisticBlockRequest";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendOptimisticBlockResponse {}
impl ::prost::Name for SendOptimisticBlockResponse {
    const NAME: &'static str = "SendOptimisticBlockResponse";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendFinalizedHashRequest {
    #[prost(bytes = "bytes", tag = "1")]
    pub block_hash: ::prost::bytes::Bytes,
}
impl ::prost::Name for SendFinalizedHashRequest {
    const NAME: &'static str = "SendFinalizedHashRequest";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendFinalizedHashResponse {}
impl ::prost::Name for SendFinalizedHashResponse {
    const NAME: &'static str = "SendFinalizedHashResponse";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod grpc_collector_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// GrpcCollectorService is a service that defines the gRPC collector of the Composer
    #[derive(Debug, Clone)]
    pub struct GrpcCollectorServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl GrpcCollectorServiceClient<tonic::transport::Channel> {
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
    impl<T> GrpcCollectorServiceClient<T>
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
        ) -> GrpcCollectorServiceClient<InterceptedService<T, F>>
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
            GrpcCollectorServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// SubmitRollupTransaction submits a rollup transactions to the Composer.
        /// The transaction sent is bundled up with other transactions and submitted to the Shared Sequencer Network.
        pub async fn submit_rollup_transaction(
            &mut self,
            request: impl tonic::IntoRequest<super::SubmitRollupTransactionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SubmitRollupTransactionResponse>,
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
                "/astria.composer.v1alpha1.GrpcCollectorService/SubmitRollupTransaction",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.composer.v1alpha1.GrpcCollectorService",
                        "SubmitRollupTransaction",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod sequencer_hooks_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct SequencerHooksServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SequencerHooksServiceClient<tonic::transport::Channel> {
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
    impl<T> SequencerHooksServiceClient<T>
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
        ) -> SequencerHooksServiceClient<InterceptedService<T, F>>
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
            SequencerHooksServiceClient::new(InterceptedService::new(inner, interceptor))
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
        pub async fn send_optimistic_block(
            &mut self,
            request: impl tonic::IntoRequest<super::SendOptimisticBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SendOptimisticBlockResponse>,
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
                "/astria.composer.v1alpha1.SequencerHooksService/SendOptimisticBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.composer.v1alpha1.SequencerHooksService",
                        "SendOptimisticBlock",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn send_finalized_hash(
            &mut self,
            request: impl tonic::IntoRequest<super::SendFinalizedHashRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SendFinalizedHashResponse>,
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
                "/astria.composer.v1alpha1.SequencerHooksService/SendFinalizedHash",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.composer.v1alpha1.SequencerHooksService",
                        "SendFinalizedHash",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod grpc_collector_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with GrpcCollectorServiceServer.
    #[async_trait]
    pub trait GrpcCollectorService: Send + Sync + 'static {
        /// SubmitRollupTransaction submits a rollup transactions to the Composer.
        /// The transaction sent is bundled up with other transactions and submitted to the Shared Sequencer Network.
        async fn submit_rollup_transaction(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::SubmitRollupTransactionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SubmitRollupTransactionResponse>,
            tonic::Status,
        >;
    }
    /// GrpcCollectorService is a service that defines the gRPC collector of the Composer
    #[derive(Debug)]
    pub struct GrpcCollectorServiceServer<T: GrpcCollectorService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: GrpcCollectorService> GrpcCollectorServiceServer<T> {
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
    for GrpcCollectorServiceServer<T>
    where
        T: GrpcCollectorService,
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
                "/astria.composer.v1alpha1.GrpcCollectorService/SubmitRollupTransaction" => {
                    #[allow(non_camel_case_types)]
                    struct SubmitRollupTransactionSvc<T: GrpcCollectorService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: GrpcCollectorService,
                    > tonic::server::UnaryService<super::SubmitRollupTransactionRequest>
                    for SubmitRollupTransactionSvc<T> {
                        type Response = super::SubmitRollupTransactionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::SubmitRollupTransactionRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as GrpcCollectorService>::submit_rollup_transaction(
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
                        let method = SubmitRollupTransactionSvc(inner);
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
    impl<T: GrpcCollectorService> Clone for GrpcCollectorServiceServer<T> {
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
    impl<T: GrpcCollectorService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: GrpcCollectorService> tonic::server::NamedService
    for GrpcCollectorServiceServer<T> {
        const NAME: &'static str = "astria.composer.v1alpha1.GrpcCollectorService";
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod sequencer_hooks_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SequencerHooksServiceServer.
    #[async_trait]
    pub trait SequencerHooksService: Send + Sync + 'static {
        async fn send_optimistic_block(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::SendOptimisticBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SendOptimisticBlockResponse>,
            tonic::Status,
        >;
        async fn send_finalized_hash(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::SendFinalizedHashRequest>,
        ) -> std::result::Result<
            tonic::Response<super::SendFinalizedHashResponse>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct SequencerHooksServiceServer<T: SequencerHooksService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: SequencerHooksService> SequencerHooksServiceServer<T> {
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
    for SequencerHooksServiceServer<T>
    where
        T: SequencerHooksService,
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
                "/astria.composer.v1alpha1.SequencerHooksService/SendOptimisticBlock" => {
                    #[allow(non_camel_case_types)]
                    struct SendOptimisticBlockSvc<T: SequencerHooksService>(pub Arc<T>);
                    impl<
                        T: SequencerHooksService,
                    > tonic::server::UnaryService<super::SendOptimisticBlockRequest>
                    for SendOptimisticBlockSvc<T> {
                        type Response = super::SendOptimisticBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SendOptimisticBlockRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SequencerHooksService>::send_optimistic_block(
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
                        let method = SendOptimisticBlockSvc(inner);
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
                "/astria.composer.v1alpha1.SequencerHooksService/SendFinalizedHash" => {
                    #[allow(non_camel_case_types)]
                    struct SendFinalizedHashSvc<T: SequencerHooksService>(pub Arc<T>);
                    impl<
                        T: SequencerHooksService,
                    > tonic::server::UnaryService<super::SendFinalizedHashRequest>
                    for SendFinalizedHashSvc<T> {
                        type Response = super::SendFinalizedHashResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SendFinalizedHashRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SequencerHooksService>::send_finalized_hash(
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
                        let method = SendFinalizedHashSvc(inner);
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
    impl<T: SequencerHooksService> Clone for SequencerHooksServiceServer<T> {
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
    impl<T: SequencerHooksService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: SequencerHooksService> tonic::server::NamedService
    for SequencerHooksServiceServer<T> {
        const NAME: &'static str = "astria.composer.v1alpha1.SequencerHooksService";
    }
}
/// BuilderBundle contains a bundle of RollupData transactions which are created by a trusted builder
/// It contains the transactions and the parent hash on top of which the bundles were simulated.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BuilderBundle {
    /// transactions in the bundle
    #[prost(message, repeated, tag = "1")]
    pub transactions: ::prost::alloc::vec::Vec<
        super::super::sequencerblock::v1alpha1::RollupData,
    >,
    /// parent hash of the bundle
    #[prost(bytes = "bytes", tag = "2")]
    pub parent_hash: ::prost::bytes::Bytes,
}
impl ::prost::Name for BuilderBundle {
    const NAME: &'static str = "BuilderBundle";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
/// BuilderBundlePacket is a message that represents a bundle of RollupData transactions and the signature
/// of the BuilderBundle by the trusted builder.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BuilderBundlePacket {
    /// the bundle of transactions
    #[prost(message, optional, tag = "1")]
    pub bundle: ::core::option::Option<BuilderBundle>,
    /// the signature of the bundle signed by the trusted builder
    #[prost(bytes = "bytes", tag = "3")]
    pub signature: ::prost::bytes::Bytes,
}
impl ::prost::Name for BuilderBundlePacket {
    const NAME: &'static str = "BuilderBundlePacket";
    const PACKAGE: &'static str = "astria.composer.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.composer.v1alpha1.{}", Self::NAME)
    }
}
