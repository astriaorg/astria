/// QuotePrice is the representation of the aggregated prices for a CurrencyPair,
/// where price represents the price of Base in terms of Quote
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QuotePrice {
    #[prost(string, tag = "1")]
    pub price: ::prost::alloc::string::String,
    /// BlockTimestamp tracks the block height associated with this price update.
    /// We include block timestamp alongside the price to ensure that smart
    /// contracts and applications are not utilizing stale oracle prices
    #[prost(message, optional, tag = "2")]
    pub block_timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
    /// BlockHeight is height of block mentioned above
    #[prost(uint64, tag = "3")]
    pub block_height: u64,
}
impl ::prost::Name for QuotePrice {
    const NAME: &'static str = "QuotePrice";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// CurrencyPairState represents the stateful information tracked by the x/oracle
/// module per-currency-pair.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrencyPairState {
    /// QuotePrice is the latest price for a currency-pair, notice this value can
    /// be null in the case that no price exists for the currency-pair
    #[prost(message, optional, tag = "1")]
    pub price: ::core::option::Option<QuotePrice>,
    /// Nonce is the number of updates this currency-pair has received
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// ID is the ID of the CurrencyPair
    #[prost(uint64, tag = "3")]
    pub id: u64,
}
impl ::prost::Name for CurrencyPairState {
    const NAME: &'static str = "CurrencyPairState";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// CurrencyPairGenesis is the information necessary for initialization of a
/// CurrencyPair.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrencyPairGenesis {
    /// The CurrencyPair to be added to module state
    #[prost(message, optional, tag = "1")]
    pub currency_pair: ::core::option::Option<super::super::types::v1::CurrencyPair>,
    /// A genesis price if one exists (note this will be empty, unless it results
    /// from forking the state of this module)
    #[prost(message, optional, tag = "2")]
    pub currency_pair_price: ::core::option::Option<QuotePrice>,
    /// nonce is the nonce (number of updates) for the CP (same case as above,
    /// likely 0 unless it results from fork of module)
    #[prost(uint64, tag = "3")]
    pub nonce: u64,
    /// id is the ID of the CurrencyPair
    #[prost(uint64, tag = "4")]
    pub id: u64,
}
impl ::prost::Name for CurrencyPairGenesis {
    const NAME: &'static str = "CurrencyPairGenesis";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GenesisState is the genesis-state for the x/oracle module, it takes a set of
/// predefined CurrencyPairGeneses
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenesisState {
    /// CurrencyPairGenesis is the set of CurrencyPairGeneses for the module. I.e
    /// the starting set of CurrencyPairs for the module + information regarding
    /// their latest update.
    #[prost(message, repeated, tag = "1")]
    pub currency_pair_genesis: ::prost::alloc::vec::Vec<CurrencyPairGenesis>,
    /// NextID is the next ID to be used for a CurrencyPair
    #[prost(uint64, tag = "2")]
    pub next_id: u64,
}
impl ::prost::Name for GenesisState {
    const NAME: &'static str = "GenesisState";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAllCurrencyPairsRequest {}
impl ::prost::Name for GetAllCurrencyPairsRequest {
    const NAME: &'static str = "GetAllCurrencyPairsRequest";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetAllCurrencyPairsResponse returns all CurrencyPairs that the module is
/// currently tracking.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAllCurrencyPairsResponse {
    #[prost(message, repeated, tag = "1")]
    pub currency_pairs: ::prost::alloc::vec::Vec<super::super::types::v1::CurrencyPair>,
}
impl ::prost::Name for GetAllCurrencyPairsResponse {
    const NAME: &'static str = "GetAllCurrencyPairsResponse";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetPriceRequest either takes a CurrencyPair, or an identifier for the
/// CurrencyPair in the format base/quote.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPriceRequest {
    /// CurrencyPair represents the pair that the user wishes to query.
    #[prost(message, optional, tag = "1")]
    pub currency_pair: ::core::option::Option<super::super::types::v1::CurrencyPair>,
}
impl ::prost::Name for GetPriceRequest {
    const NAME: &'static str = "GetPriceRequest";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetPriceResponse is the response from the GetPrice grpc method exposed from
/// the x/oracle query service.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPriceResponse {
    /// QuotePrice represents the quote-price for the CurrencyPair given in
    /// GetPriceRequest (possibly nil if no update has been made)
    #[prost(message, optional, tag = "1")]
    pub price: ::core::option::Option<QuotePrice>,
    /// nonce represents the nonce for the CurrencyPair if it exists in state
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// decimals represents the number of decimals that the quote-price is
    /// represented in. For Pairs where ETHEREUM is the quote this will be 18,
    /// otherwise it will be 8.
    #[prost(uint64, tag = "3")]
    pub decimals: u64,
    /// ID represents the identifier for the CurrencyPair.
    #[prost(uint64, tag = "4")]
    pub id: u64,
}
impl ::prost::Name for GetPriceResponse {
    const NAME: &'static str = "GetPriceResponse";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetPricesRequest takes an identifier for the CurrencyPair
/// in the format base/quote.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPricesRequest {
    #[prost(string, repeated, tag = "1")]
    pub currency_pair_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
impl ::prost::Name for GetPricesRequest {
    const NAME: &'static str = "GetPricesRequest";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetPricesResponse is the response from the GetPrices grpc method exposed from
/// the x/oracle query service.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPricesResponse {
    #[prost(message, repeated, tag = "1")]
    pub prices: ::prost::alloc::vec::Vec<GetPriceResponse>,
}
impl ::prost::Name for GetPricesResponse {
    const NAME: &'static str = "GetPricesResponse";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetCurrencyPairMappingRequest is the GetCurrencyPairMapping request type.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCurrencyPairMappingRequest {}
impl ::prost::Name for GetCurrencyPairMappingRequest {
    const NAME: &'static str = "GetCurrencyPairMappingRequest";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// GetCurrencyPairMappingResponse is the GetCurrencyPairMapping response type.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCurrencyPairMappingResponse {
    /// currency_pair_mapping is a mapping of the id representing the currency pair
    /// to the currency pair itself.
    #[prost(map = "uint64, message", tag = "1")]
    pub currency_pair_mapping: ::std::collections::HashMap<
        u64,
        super::super::types::v1::CurrencyPair,
    >,
}
impl ::prost::Name for GetCurrencyPairMappingResponse {
    const NAME: &'static str = "GetCurrencyPairMappingResponse";
    const PACKAGE: &'static str = "astria_vendored.slinky.oracle.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria_vendored.slinky.oracle.v1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod query_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Query is the query service for the x/oracle module.
    #[derive(Debug, Clone)]
    pub struct QueryClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl QueryClient<tonic::transport::Channel> {
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
    impl<T> QueryClient<T>
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
        ) -> QueryClient<InterceptedService<T, F>>
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
            QueryClient::new(InterceptedService::new(inner, interceptor))
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
        /// Get all the currency pairs the x/oracle module is tracking price-data for.
        pub async fn get_all_currency_pairs(
            &mut self,
            request: impl tonic::IntoRequest<super::GetAllCurrencyPairsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetAllCurrencyPairsResponse>,
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
                "/astria_vendored.slinky.oracle.v1.Query/GetAllCurrencyPairs",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria_vendored.slinky.oracle.v1.Query",
                        "GetAllCurrencyPairs",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Given a CurrencyPair (or its identifier) return the latest QuotePrice for
        /// that CurrencyPair.
        pub async fn get_price(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPriceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPriceResponse>,
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
                "/astria_vendored.slinky.oracle.v1.Query/GetPrice",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("astria_vendored.slinky.oracle.v1.Query", "GetPrice"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_prices(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPricesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPricesResponse>,
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
                "/astria_vendored.slinky.oracle.v1.Query/GetPrices",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria_vendored.slinky.oracle.v1.Query",
                        "GetPrices",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Get the mapping of currency pair ID -> currency pair. This is useful for
        /// indexers that have access to the ID of a currency pair, but no way to get
        /// the underlying currency pair from it.
        pub async fn get_currency_pair_mapping(
            &mut self,
            request: impl tonic::IntoRequest<super::GetCurrencyPairMappingRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetCurrencyPairMappingResponse>,
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
                "/astria_vendored.slinky.oracle.v1.Query/GetCurrencyPairMapping",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "astria_vendored.slinky.oracle.v1.Query",
                        "GetCurrencyPairMapping",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod query_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with QueryServer.
    #[async_trait]
    pub trait Query: Send + Sync + 'static {
        /// Get all the currency pairs the x/oracle module is tracking price-data for.
        async fn get_all_currency_pairs(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetAllCurrencyPairsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetAllCurrencyPairsResponse>,
            tonic::Status,
        >;
        /// Given a CurrencyPair (or its identifier) return the latest QuotePrice for
        /// that CurrencyPair.
        async fn get_price(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetPriceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPriceResponse>,
            tonic::Status,
        >;
        async fn get_prices(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetPricesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPricesResponse>,
            tonic::Status,
        >;
        /// Get the mapping of currency pair ID -> currency pair. This is useful for
        /// indexers that have access to the ID of a currency pair, but no way to get
        /// the underlying currency pair from it.
        async fn get_currency_pair_mapping(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::GetCurrencyPairMappingRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetCurrencyPairMappingResponse>,
            tonic::Status,
        >;
    }
    /// Query is the query service for the x/oracle module.
    #[derive(Debug)]
    pub struct QueryServer<T: Query> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Query> QueryServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for QueryServer<T>
    where
        T: Query,
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
                "/astria_vendored.slinky.oracle.v1.Query/GetAllCurrencyPairs" => {
                    #[allow(non_camel_case_types)]
                    struct GetAllCurrencyPairsSvc<T: Query>(pub Arc<T>);
                    impl<
                        T: Query,
                    > tonic::server::UnaryService<super::GetAllCurrencyPairsRequest>
                    for GetAllCurrencyPairsSvc<T> {
                        type Response = super::GetAllCurrencyPairsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetAllCurrencyPairsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::get_all_currency_pairs(inner, request).await
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
                        let method = GetAllCurrencyPairsSvc(inner);
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
                "/astria_vendored.slinky.oracle.v1.Query/GetPrice" => {
                    #[allow(non_camel_case_types)]
                    struct GetPriceSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::GetPriceRequest>
                    for GetPriceSvc<T> {
                        type Response = super::GetPriceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPriceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::get_price(inner, request).await
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
                        let method = GetPriceSvc(inner);
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
                "/astria_vendored.slinky.oracle.v1.Query/GetPrices" => {
                    #[allow(non_camel_case_types)]
                    struct GetPricesSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::GetPricesRequest>
                    for GetPricesSvc<T> {
                        type Response = super::GetPricesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPricesRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::get_prices(inner, request).await
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
                        let method = GetPricesSvc(inner);
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
                "/astria_vendored.slinky.oracle.v1.Query/GetCurrencyPairMapping" => {
                    #[allow(non_camel_case_types)]
                    struct GetCurrencyPairMappingSvc<T: Query>(pub Arc<T>);
                    impl<
                        T: Query,
                    > tonic::server::UnaryService<super::GetCurrencyPairMappingRequest>
                    for GetCurrencyPairMappingSvc<T> {
                        type Response = super::GetCurrencyPairMappingResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetCurrencyPairMappingRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::get_currency_pair_mapping(inner, request)
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
                        let method = GetCurrencyPairMappingSvc(inner);
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
    impl<T: Query> Clone for QueryServer<T> {
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
    impl<T: Query> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Query> tonic::server::NamedService for QueryServer<T> {
        const NAME: &'static str = "astria_vendored.slinky.oracle.v1.Query";
    }
}
