/// The Allocation message is submitted by the Auctioneer to the rollup as a
/// `RollupDataSubmission` on the sequencer.
/// The rollup will verify the signature and public key against its configuration,
/// then unbundle the body into rollup transactions and execute them first in the
/// block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Allocation {
    /// The Ed25519 signature of the Auctioneer, to be verified against config by the
    /// rollup.
    #[prost(bytes = "bytes", tag = "1")]
    pub signature: ::prost::bytes::Bytes,
    /// The Ed25519 public key of the Auctioneer, to be verified against config by the
    /// rollup.
    #[prost(bytes = "bytes", tag = "2")]
    pub public_key: ::prost::bytes::Bytes,
    /// The bid that was allocated the winning slot by the Auctioneer. This is a
    /// google.protobuf.Any to avoid decoding and re-encoding after receiving an Allocation
    /// over the wire and checking if signature and public key match the signed bid.
    /// Implementors are expected to read and write an encoded Bid into this field.
    #[prost(message, optional, tag = "3")]
    pub bid: ::core::option::Option<::pbjson_types::Any>,
}
impl ::prost::Name for Allocation {
    const NAME: &'static str = "Allocation";
    const PACKAGE: &'static str = "astria.auction.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.auction.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBidStreamRequest {}
impl ::prost::Name for GetBidStreamRequest {
    const NAME: &'static str = "GetBidStreamRequest";
    const PACKAGE: &'static str = "astria.auction.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.auction.v1alpha1.{}", Self::NAME)
    }
}
/// A bid is a bundle of transactions that was submitted to the auctioneer's rollup node.
/// The rollup node will verify that the bundle is valid and pays the fee, and will stream
/// it to the auctioneer for participation in the auction for a given block.
/// The sequencer block hash and the rollup parent block hash are used by the auctioneer
/// to identify the block for which the bundle is intended (i.e. which auction the bid is for).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Bid {
    /// The hash of previous rollup block, on top of which the bundle will be executed as ToB.
    #[prost(bytes = "bytes", tag = "1")]
    pub rollup_parent_block_hash: ::prost::bytes::Bytes,
    /// The hash of the previous sequencer block, identifying the auction for which the bid is intended.
    /// This is the hash of the sequencer block on top of which the bundle will be executed as ToB.
    #[prost(bytes = "bytes", tag = "2")]
    pub sequencer_parent_block_hash: ::prost::bytes::Bytes,
    /// The fee paid by the bundle submitter. The auctioneer's rollup node calculates this based
    /// on the bundles submitted by users. For example, this can be the sum of the coinbase transfers
    /// in the bundle's transactions.
    #[prost(uint64, tag = "3")]
    pub fee: u64,
    /// The list of serialized rollup transactions from the bundle.
    #[prost(bytes = "bytes", repeated, tag = "4")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::bytes::Bytes>,
}
impl ::prost::Name for Bid {
    const NAME: &'static str = "Bid";
    const PACKAGE: &'static str = "astria.auction.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.auction.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBidStreamResponse {
    #[prost(message, optional, tag = "1")]
    pub bid: ::core::option::Option<Bid>,
}
impl ::prost::Name for GetBidStreamResponse {
    const NAME: &'static str = "GetBidStreamResponse";
    const PACKAGE: &'static str = "astria.auction.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.auction.v1alpha1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod auction_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct AuctionServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl AuctionServiceClient<tonic::transport::Channel> {
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
    impl<T> AuctionServiceClient<T>
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
        ) -> AuctionServiceClient<InterceptedService<T, F>>
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
            AuctionServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// An auctioneer will initiate this long running stream to receive bids from the rollup node,
        /// until either a timeout or the connection is closed by the client.
        pub async fn get_bid_stream(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBidStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::GetBidStreamResponse>>,
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
                "/astria.auction.v1alpha1.AuctionService/GetBidStream",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.auction.v1alpha1.AuctionService",
                        "GetBidStream",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod auction_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with AuctionServiceServer.
    #[async_trait]
    pub trait AuctionService: Send + Sync + 'static {
        /// Server streaming response type for the GetBidStream method.
        type GetBidStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<super::GetBidStreamResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// An auctioneer will initiate this long running stream to receive bids from the rollup node,
        /// until either a timeout or the connection is closed by the client.
        async fn get_bid_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetBidStreamRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::GetBidStreamStream>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct AuctionServiceServer<T: AuctionService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: AuctionService> AuctionServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for AuctionServiceServer<T>
    where
        T: AuctionService,
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
                "/astria.auction.v1alpha1.AuctionService/GetBidStream" => {
                    #[allow(non_camel_case_types)]
                    struct GetBidStreamSvc<T: AuctionService>(pub Arc<T>);
                    impl<
                        T: AuctionService,
                    > tonic::server::ServerStreamingService<super::GetBidStreamRequest>
                    for GetBidStreamSvc<T> {
                        type Response = super::GetBidStreamResponse;
                        type ResponseStream = T::GetBidStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBidStreamRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuctionService>::get_bid_stream(inner, request).await
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
                        let method = GetBidStreamSvc(inner);
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
    impl<T: AuctionService> Clone for AuctionServiceServer<T> {
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
    impl<T: AuctionService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: AuctionService> tonic::server::NamedService for AuctionServiceServer<T> {
        const NAME: &'static str = "astria.auction.v1alpha1.AuctionService";
    }
}
