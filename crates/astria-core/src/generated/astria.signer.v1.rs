#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitmentWithIdentifier {
    #[prost(bytes = "bytes", tag = "1")]
    pub commitment: ::prost::bytes::Bytes,
    #[prost(bytes = "bytes", tag = "2")]
    pub participant_identifier: ::prost::bytes::Bytes,
}
impl ::prost::Name for CommitmentWithIdentifier {
    const NAME: &'static str = "CommitmentWithIdentifier";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetVerifyingShareRequest {}
impl ::prost::Name for GetVerifyingShareRequest {
    const NAME: &'static str = "GetVerifyingShareRequest";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteRoundOneRequest {}
impl ::prost::Name for ExecuteRoundOneRequest {
    const NAME: &'static str = "ExecuteRoundOneRequest";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RoundOneResponse {
    #[prost(bytes = "bytes", tag = "1")]
    pub commitment: ::prost::bytes::Bytes,
    /// required for the participant to internally track the nonce
    /// corresponding to the commitment.
    #[prost(uint32, tag = "2")]
    pub request_identifier: u32,
}
impl ::prost::Name for RoundOneResponse {
    const NAME: &'static str = "RoundOneResponse";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecuteRoundTwoRequest {
    #[prost(message, repeated, tag = "1")]
    pub commitments: ::prost::alloc::vec::Vec<CommitmentWithIdentifier>,
    #[prost(bytes = "bytes", tag = "2")]
    pub message: ::prost::bytes::Bytes,
    #[prost(uint32, tag = "3")]
    pub request_identifier: u32,
}
impl ::prost::Name for ExecuteRoundTwoRequest {
    const NAME: &'static str = "ExecuteRoundTwoRequest";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RoundTwoResponse {
    #[prost(bytes = "bytes", tag = "1")]
    pub signature_share: ::prost::bytes::Bytes,
}
impl ::prost::Name for RoundTwoResponse {
    const NAME: &'static str = "RoundTwoResponse";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VerifyingShare {
    /// the verifying share (partial public key) of the participant.
    /// this is used for the coordinator to determine the identifier of the participant.
    /// TODO: do we need to verify this (ie. have the server send back a signed message
    /// with the verifying share)?
    #[prost(bytes = "bytes", tag = "1")]
    pub verifying_share: ::prost::bytes::Bytes,
}
impl ::prost::Name for VerifyingShare {
    const NAME: &'static str = "VerifyingShare";
    const PACKAGE: &'static str = "astria.signer.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.signer.v1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod frost_participant_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct FrostParticipantServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl FrostParticipantServiceClient<tonic::transport::Channel> {
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
    impl<T> FrostParticipantServiceClient<T>
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
        ) -> FrostParticipantServiceClient<InterceptedService<T, F>>
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
            FrostParticipantServiceClient::new(
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
        pub async fn get_verifying_share(
            &mut self,
            request: impl tonic::IntoRequest<super::GetVerifyingShareRequest>,
        ) -> std::result::Result<tonic::Response<super::VerifyingShare>, tonic::Status> {
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
                "/astria.signer.v1.FrostParticipantService/GetVerifyingShare",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.signer.v1.FrostParticipantService",
                        "GetVerifyingShare",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn execute_round_one(
            &mut self,
            request: impl tonic::IntoRequest<super::ExecuteRoundOneRequest>,
        ) -> std::result::Result<
            tonic::Response<super::RoundOneResponse>,
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
                "/astria.signer.v1.FrostParticipantService/ExecuteRoundOne",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.signer.v1.FrostParticipantService",
                        "ExecuteRoundOne",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn execute_round_two(
            &mut self,
            request: impl tonic::IntoRequest<super::ExecuteRoundTwoRequest>,
        ) -> std::result::Result<
            tonic::Response<super::RoundTwoResponse>,
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
                "/astria.signer.v1.FrostParticipantService/ExecuteRoundTwo",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria.signer.v1.FrostParticipantService",
                        "ExecuteRoundTwo",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod frost_participant_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with FrostParticipantServiceServer.
    #[async_trait]
    pub trait FrostParticipantService: Send + Sync + 'static {
        async fn get_verifying_share(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetVerifyingShareRequest>,
        ) -> std::result::Result<tonic::Response<super::VerifyingShare>, tonic::Status>;
        async fn execute_round_one(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::ExecuteRoundOneRequest>,
        ) -> std::result::Result<
            tonic::Response<super::RoundOneResponse>,
            tonic::Status,
        >;
        async fn execute_round_two(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::ExecuteRoundTwoRequest>,
        ) -> std::result::Result<
            tonic::Response<super::RoundTwoResponse>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct FrostParticipantServiceServer<T: FrostParticipantService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: FrostParticipantService> FrostParticipantServiceServer<T> {
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
    for FrostParticipantServiceServer<T>
    where
        T: FrostParticipantService,
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
                "/astria.signer.v1.FrostParticipantService/GetVerifyingShare" => {
                    #[allow(non_camel_case_types)]
                    struct GetVerifyingShareSvc<T: FrostParticipantService>(pub Arc<T>);
                    impl<
                        T: FrostParticipantService,
                    > tonic::server::UnaryService<super::GetVerifyingShareRequest>
                    for GetVerifyingShareSvc<T> {
                        type Response = super::VerifyingShare;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetVerifyingShareRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as FrostParticipantService>::get_verifying_share(
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
                        let method = GetVerifyingShareSvc(inner);
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
                "/astria.signer.v1.FrostParticipantService/ExecuteRoundOne" => {
                    #[allow(non_camel_case_types)]
                    struct ExecuteRoundOneSvc<T: FrostParticipantService>(pub Arc<T>);
                    impl<
                        T: FrostParticipantService,
                    > tonic::server::UnaryService<super::ExecuteRoundOneRequest>
                    for ExecuteRoundOneSvc<T> {
                        type Response = super::RoundOneResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ExecuteRoundOneRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as FrostParticipantService>::execute_round_one(
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
                        let method = ExecuteRoundOneSvc(inner);
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
                "/astria.signer.v1.FrostParticipantService/ExecuteRoundTwo" => {
                    #[allow(non_camel_case_types)]
                    struct ExecuteRoundTwoSvc<T: FrostParticipantService>(pub Arc<T>);
                    impl<
                        T: FrostParticipantService,
                    > tonic::server::UnaryService<super::ExecuteRoundTwoRequest>
                    for ExecuteRoundTwoSvc<T> {
                        type Response = super::RoundTwoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ExecuteRoundTwoRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as FrostParticipantService>::execute_round_two(
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
                        let method = ExecuteRoundTwoSvc(inner);
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
    impl<T: FrostParticipantService> Clone for FrostParticipantServiceServer<T> {
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
    impl<T: FrostParticipantService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: FrostParticipantService> tonic::server::NamedService
    for FrostParticipantServiceServer<T> {
        const NAME: &'static str = "astria.signer.v1.FrostParticipantService";
    }
}
