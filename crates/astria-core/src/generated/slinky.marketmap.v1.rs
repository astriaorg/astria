/// Market encapsulates a Ticker and its provider-specific configuration.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Market {
    /// Ticker represents a price feed for a given asset pair i.e. BTC/USD. The
    /// price feed is scaled to a number of decimal places and has a minimum number
    /// of providers required to consider the ticker valid.
    #[prost(message, optional, tag = "1")]
    pub ticker: ::core::option::Option<Ticker>,
    /// ProviderConfigs is the list of provider-specific configs for this Market.
    #[prost(message, repeated, tag = "2")]
    pub provider_configs: ::prost::alloc::vec::Vec<ProviderConfig>,
}
impl ::prost::Name for Market {
    const NAME: &'static str = "Market";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// Ticker represents a price feed for a given asset pair i.e. BTC/USD. The price
/// feed is scaled to a number of decimal places and has a minimum number of
/// providers required to consider the ticker valid.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ticker {
    /// CurrencyPair is the currency pair for this ticker.
    #[prost(message, optional, tag = "1")]
    pub currency_pair: ::core::option::Option<super::super::types::v1::CurrencyPair>,
    /// Decimals is the number of decimal places for the ticker. The number of
    /// decimal places is used to convert the price to a human-readable format.
    #[prost(uint64, tag = "2")]
    pub decimals: u64,
    /// MinProviderCount is the minimum number of providers required to consider
    /// the ticker valid.
    #[prost(uint64, tag = "3")]
    pub min_provider_count: u64,
    /// Enabled is the flag that denotes if the Ticker is enabled for price
    /// fetching by an oracle.
    #[prost(bool, tag = "14")]
    pub enabled: bool,
    /// MetadataJSON is a string of JSON that encodes any extra configuration
    /// for the given ticker.
    #[prost(string, tag = "15")]
    pub metadata_json: ::prost::alloc::string::String,
}
impl ::prost::Name for Ticker {
    const NAME: &'static str = "Ticker";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProviderConfig {
    /// Name corresponds to the name of the provider for which the configuration is
    /// being set.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// OffChainTicker is the off-chain representation of the ticker i.e. BTC/USD.
    /// The off-chain ticker is unique to a given provider and is used to fetch the
    /// price of the ticker from the provider.
    #[prost(string, tag = "2")]
    pub off_chain_ticker: ::prost::alloc::string::String,
    /// NormalizeByPair is the currency pair for this ticker to be normalized by.
    /// For example, if the desired Ticker is BTC/USD, this market could be reached
    /// using: OffChainTicker = BTC/USDT NormalizeByPair = USDT/USD This field is
    /// optional and nullable.
    #[prost(message, optional, tag = "3")]
    pub normalize_by_pair: ::core::option::Option<super::super::types::v1::CurrencyPair>,
    /// Invert is a boolean indicating if the BASE and QUOTE of the market should
    /// be inverted. i.e. BASE -> QUOTE, QUOTE -> BASE
    #[prost(bool, tag = "4")]
    pub invert: bool,
    /// MetadataJSON is a string of JSON that encodes any extra configuration
    /// for the given provider config.
    #[prost(string, tag = "15")]
    pub metadata_json: ::prost::alloc::string::String,
}
impl ::prost::Name for ProviderConfig {
    const NAME: &'static str = "ProviderConfig";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MarketMap maps ticker strings to their Markets.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MarketMap {
    /// Markets is the full list of tickers and their associated configurations
    /// to be stored on-chain.
    #[prost(map = "string, message", tag = "1")]
    pub markets: ::std::collections::HashMap<::prost::alloc::string::String, Market>,
}
impl ::prost::Name for MarketMap {
    const NAME: &'static str = "MarketMap";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// Params defines the parameters for the x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Params {
    /// MarketAuthorities is the list of authority accounts that are able to
    /// control updating the marketmap.
    #[prost(string, repeated, tag = "1")]
    pub market_authorities: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Admin is an address that can remove addresses from the MarketAuthorities
    /// list. Only governance can add to the MarketAuthorities or change the Admin.
    #[prost(string, tag = "2")]
    pub admin: ::prost::alloc::string::String,
}
impl ::prost::Name for Params {
    const NAME: &'static str = "Params";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// GenesisState defines the x/marketmap module's genesis state.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenesisState {
    /// MarketMap defines the global set of market configurations for all providers
    /// and markets.
    #[prost(message, optional, tag = "1")]
    pub market_map: ::core::option::Option<MarketMap>,
    /// LastUpdated is the last block height that the market map was updated.
    /// This field can be used as an optimization for clients checking if there
    /// is a new update to the map.
    #[prost(uint64, tag = "2")]
    pub last_updated: u64,
    /// Params are the parameters for the x/marketmap module.
    #[prost(message, optional, tag = "3")]
    pub params: ::core::option::Option<Params>,
}
impl ::prost::Name for GenesisState {
    const NAME: &'static str = "GenesisState";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MarketMapRequest is the query request for the MarketMap query.
/// It takes no arguments.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MarketMapRequest {}
impl ::prost::Name for MarketMapRequest {
    const NAME: &'static str = "MarketMapRequest";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MarketMapResponse is the query response for the MarketMap query.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MarketMapResponse {
    /// MarketMap defines the global set of market configurations for all providers
    /// and markets.
    #[prost(message, optional, tag = "1")]
    pub market_map: ::core::option::Option<MarketMap>,
    /// LastUpdated is the last block height that the market map was updated.
    /// This field can be used as an optimization for clients checking if there
    /// is a new update to the map.
    #[prost(uint64, tag = "2")]
    pub last_updated: u64,
    /// ChainId is the chain identifier for the market map.
    #[prost(string, tag = "3")]
    pub chain_id: ::prost::alloc::string::String,
}
impl ::prost::Name for MarketMapResponse {
    const NAME: &'static str = "MarketMapResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MarketRequest is the query request for the Market query.
/// It takes the currency pair of the market as an argument.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MarketRequest {
    /// CurrencyPair is the currency pair associated with the market being
    /// requested.
    #[prost(message, optional, tag = "1")]
    pub currency_pair: ::core::option::Option<super::super::types::v1::CurrencyPair>,
}
impl ::prost::Name for MarketRequest {
    const NAME: &'static str = "MarketRequest";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MarketResponse is the query response for the Market query.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MarketResponse {
    /// Market is the configuration of a single market to be price-fetched for.
    #[prost(message, optional, tag = "1")]
    pub market: ::core::option::Option<Market>,
}
impl ::prost::Name for MarketResponse {
    const NAME: &'static str = "MarketResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// ParamsRequest is the request type for the Query/Params RPC method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ParamsRequest {}
impl ::prost::Name for ParamsRequest {
    const NAME: &'static str = "ParamsRequest";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// ParamsResponse is the response type for the Query/Params RPC method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ParamsResponse {
    #[prost(message, optional, tag = "1")]
    pub params: ::core::option::Option<Params>,
}
impl ::prost::Name for ParamsResponse {
    const NAME: &'static str = "ParamsResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// LastUpdatedRequest is the request type for the Query/LastUpdated RPC
/// method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LastUpdatedRequest {}
impl ::prost::Name for LastUpdatedRequest {
    const NAME: &'static str = "LastUpdatedRequest";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// LastUpdatedResponse is the response type for the Query/LastUpdated RPC
/// method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LastUpdatedResponse {
    #[prost(uint64, tag = "1")]
    pub last_updated: u64,
}
impl ::prost::Name for LastUpdatedResponse {
    const NAME: &'static str = "LastUpdatedResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod query_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Query is the query service for the x/marketmap module.
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
        /// MarketMap returns the full market map stored in the x/marketmap
        /// module.
        pub async fn market_map(
            &mut self,
            request: impl tonic::IntoRequest<super::MarketMapRequest>,
        ) -> std::result::Result<
            tonic::Response<super::MarketMapResponse>,
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
                "/slinky.marketmap.v1.Query/MarketMap",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Query", "MarketMap"));
            self.inner.unary(req, path, codec).await
        }
        /// Market returns a market stored in the x/marketmap
        /// module.
        pub async fn market(
            &mut self,
            request: impl tonic::IntoRequest<super::MarketRequest>,
        ) -> std::result::Result<tonic::Response<super::MarketResponse>, tonic::Status> {
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
                "/slinky.marketmap.v1.Query/Market",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Query", "Market"));
            self.inner.unary(req, path, codec).await
        }
        /// LastUpdated returns the last height the market map was updated at.
        pub async fn last_updated(
            &mut self,
            request: impl tonic::IntoRequest<super::LastUpdatedRequest>,
        ) -> std::result::Result<
            tonic::Response<super::LastUpdatedResponse>,
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
                "/slinky.marketmap.v1.Query/LastUpdated",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Query", "LastUpdated"));
            self.inner.unary(req, path, codec).await
        }
        /// Params returns the current x/marketmap module parameters.
        pub async fn params(
            &mut self,
            request: impl tonic::IntoRequest<super::ParamsRequest>,
        ) -> std::result::Result<tonic::Response<super::ParamsResponse>, tonic::Status> {
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
                "/slinky.marketmap.v1.Query/Params",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Query", "Params"));
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
        /// MarketMap returns the full market map stored in the x/marketmap
        /// module.
        async fn market_map(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MarketMapRequest>,
        ) -> std::result::Result<
            tonic::Response<super::MarketMapResponse>,
            tonic::Status,
        >;
        /// Market returns a market stored in the x/marketmap
        /// module.
        async fn market(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MarketRequest>,
        ) -> std::result::Result<tonic::Response<super::MarketResponse>, tonic::Status>;
        /// LastUpdated returns the last height the market map was updated at.
        async fn last_updated(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::LastUpdatedRequest>,
        ) -> std::result::Result<
            tonic::Response<super::LastUpdatedResponse>,
            tonic::Status,
        >;
        /// Params returns the current x/marketmap module parameters.
        async fn params(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::ParamsRequest>,
        ) -> std::result::Result<tonic::Response<super::ParamsResponse>, tonic::Status>;
    }
    /// Query is the query service for the x/marketmap module.
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
                "/slinky.marketmap.v1.Query/MarketMap" => {
                    #[allow(non_camel_case_types)]
                    struct MarketMapSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::MarketMapRequest>
                    for MarketMapSvc<T> {
                        type Response = super::MarketMapResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MarketMapRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::market_map(inner, request).await
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
                        let method = MarketMapSvc(inner);
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
                "/slinky.marketmap.v1.Query/Market" => {
                    #[allow(non_camel_case_types)]
                    struct MarketSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::MarketRequest>
                    for MarketSvc<T> {
                        type Response = super::MarketResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MarketRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::market(inner, request).await
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
                        let method = MarketSvc(inner);
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
                "/slinky.marketmap.v1.Query/LastUpdated" => {
                    #[allow(non_camel_case_types)]
                    struct LastUpdatedSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::LastUpdatedRequest>
                    for LastUpdatedSvc<T> {
                        type Response = super::LastUpdatedResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::LastUpdatedRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::last_updated(inner, request).await
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
                        let method = LastUpdatedSvc(inner);
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
                "/slinky.marketmap.v1.Query/Params" => {
                    #[allow(non_camel_case_types)]
                    struct ParamsSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::ParamsRequest>
                    for ParamsSvc<T> {
                        type Response = super::ParamsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ParamsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Query>::params(inner, request).await
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
                        let method = ParamsSvc(inner);
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
        const NAME: &'static str = "slinky.marketmap.v1.Query";
    }
}
/// MsgUpsertMarkets defines a message carrying a payload for performing market upserts (update or
/// create if does not exist) in the x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpsertMarkets {
    /// Authority is the signer of this transaction.  This authority must be
    /// authorized by the module to execute the message.
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
    /// CreateMarkets is the list of all markets to be created for the given
    /// transaction.
    #[prost(message, repeated, tag = "2")]
    pub markets: ::prost::alloc::vec::Vec<Market>,
}
impl ::prost::Name for MsgUpsertMarkets {
    const NAME: &'static str = "MsgUpsertMarkets";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgUpsertMarketsResponse is the response from the UpsertMarkets API in the x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpsertMarketsResponse {
    /// UpdatedMarkets is a map between the ticker and whether the market was updated.
    #[prost(map = "string, bool", tag = "1")]
    pub market_updates: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        bool,
    >,
}
impl ::prost::Name for MsgUpsertMarketsResponse {
    const NAME: &'static str = "MsgUpsertMarketsResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgCreateMarkets defines a message carrying a payload for creating markets in
/// the x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgCreateMarkets {
    /// Authority is the signer of this transaction.  This authority must be
    /// authorized by the module to execute the message.
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
    /// CreateMarkets is the list of all markets to be created for the given
    /// transaction.
    #[prost(message, repeated, tag = "2")]
    pub create_markets: ::prost::alloc::vec::Vec<Market>,
}
impl ::prost::Name for MsgCreateMarkets {
    const NAME: &'static str = "MsgCreateMarkets";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgUpdateMarketMapResponse is the response message for MsgUpdateMarketMap.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgCreateMarketsResponse {}
impl ::prost::Name for MsgCreateMarketsResponse {
    const NAME: &'static str = "MsgCreateMarketsResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgUpdateMarkets defines a message carrying a payload for updating the
/// x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpdateMarkets {
    /// Authority is the signer of this transaction.  This authority must be
    /// authorized by the module to execute the message.
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
    /// UpdateMarkets is the list of all markets to be updated for the given
    /// transaction.
    #[prost(message, repeated, tag = "2")]
    pub update_markets: ::prost::alloc::vec::Vec<Market>,
}
impl ::prost::Name for MsgUpdateMarkets {
    const NAME: &'static str = "MsgUpdateMarkets";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgUpdateMarketsResponse is the response message for MsgUpdateMarkets.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpdateMarketsResponse {}
impl ::prost::Name for MsgUpdateMarketsResponse {
    const NAME: &'static str = "MsgUpdateMarketsResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgParams defines the Msg/Params request type. It contains the
/// new parameters for the x/marketmap module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgParams {
    /// Params defines the new parameters for the x/marketmap module.
    #[prost(message, optional, tag = "1")]
    pub params: ::core::option::Option<Params>,
    /// Authority defines the authority that is updating the x/marketmap module
    /// parameters.
    #[prost(string, tag = "2")]
    pub authority: ::prost::alloc::string::String,
}
impl ::prost::Name for MsgParams {
    const NAME: &'static str = "MsgParams";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgParamsResponse defines the Msg/Params response type.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgParamsResponse {}
impl ::prost::Name for MsgParamsResponse {
    const NAME: &'static str = "MsgParamsResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgRemoveMarketAuthorities defines the Msg/RemoveMarketAuthoritiesResponse
/// request type. It contains the new addresses to remove from the list of
/// authorities
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgRemoveMarketAuthorities {
    /// RemoveAddresses is the list of addresses to remove.
    #[prost(string, repeated, tag = "1")]
    pub remove_addresses: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Admin defines the authority that is the x/marketmap
    /// Admin account.  This account is set in the module parameters.
    #[prost(string, tag = "2")]
    pub admin: ::prost::alloc::string::String,
}
impl ::prost::Name for MsgRemoveMarketAuthorities {
    const NAME: &'static str = "MsgRemoveMarketAuthorities";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// MsgRemoveMarketAuthoritiesResponse defines the
/// Msg/RemoveMarketAuthoritiesResponse response type.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgRemoveMarketAuthoritiesResponse {}
impl ::prost::Name for MsgRemoveMarketAuthoritiesResponse {
    const NAME: &'static str = "MsgRemoveMarketAuthoritiesResponse";
    const PACKAGE: &'static str = "slinky.marketmap.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.v1.{}", Self::NAME)
    }
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod msg_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Msg is the message service for the x/marketmap module.
    #[derive(Debug, Clone)]
    pub struct MsgClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl MsgClient<tonic::transport::Channel> {
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
    impl<T> MsgClient<T>
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
        ) -> MsgClient<InterceptedService<T, F>>
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
            MsgClient::new(InterceptedService::new(inner, interceptor))
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
        /// CreateMarkets creates markets from the given message.
        pub async fn create_markets(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgCreateMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgCreateMarketsResponse>,
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
                "/slinky.marketmap.v1.Msg/CreateMarkets",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Msg", "CreateMarkets"));
            self.inner.unary(req, path, codec).await
        }
        /// UpdateMarkets updates markets from the given message.
        pub async fn update_markets(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgUpdateMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgUpdateMarketsResponse>,
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
                "/slinky.marketmap.v1.Msg/UpdateMarkets",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Msg", "UpdateMarkets"));
            self.inner.unary(req, path, codec).await
        }
        /// UpdateParams defines a method for updating the x/marketmap module
        /// parameters.
        pub async fn update_params(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgParams>,
        ) -> std::result::Result<
            tonic::Response<super::MsgParamsResponse>,
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
                "/slinky.marketmap.v1.Msg/UpdateParams",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Msg", "UpdateParams"));
            self.inner.unary(req, path, codec).await
        }
        /// RemoveMarketAuthorities defines a method for removing market authorities
        /// from the x/marketmap module. the signer must be the admin.
        pub async fn remove_market_authorities(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgRemoveMarketAuthorities>,
        ) -> std::result::Result<
            tonic::Response<super::MsgRemoveMarketAuthoritiesResponse>,
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
                "/slinky.marketmap.v1.Msg/RemoveMarketAuthorities",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("slinky.marketmap.v1.Msg", "RemoveMarketAuthorities"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// UpsertMarkets wraps both Create / Update markets into a single message. Specifically
        /// if a market does not exist it will be created, otherwise it will be updated. The response
        /// will be a map between ticker -> updated.
        pub async fn upsert_markets(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgUpsertMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgUpsertMarketsResponse>,
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
                "/slinky.marketmap.v1.Msg/UpsertMarkets",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("slinky.marketmap.v1.Msg", "UpsertMarkets"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod msg_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with MsgServer.
    #[async_trait]
    pub trait Msg: Send + Sync + 'static {
        /// CreateMarkets creates markets from the given message.
        async fn create_markets(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MsgCreateMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgCreateMarketsResponse>,
            tonic::Status,
        >;
        /// UpdateMarkets updates markets from the given message.
        async fn update_markets(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MsgUpdateMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgUpdateMarketsResponse>,
            tonic::Status,
        >;
        /// UpdateParams defines a method for updating the x/marketmap module
        /// parameters.
        async fn update_params(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MsgParams>,
        ) -> std::result::Result<
            tonic::Response<super::MsgParamsResponse>,
            tonic::Status,
        >;
        /// RemoveMarketAuthorities defines a method for removing market authorities
        /// from the x/marketmap module. the signer must be the admin.
        async fn remove_market_authorities(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MsgRemoveMarketAuthorities>,
        ) -> std::result::Result<
            tonic::Response<super::MsgRemoveMarketAuthoritiesResponse>,
            tonic::Status,
        >;
        /// UpsertMarkets wraps both Create / Update markets into a single message. Specifically
        /// if a market does not exist it will be created, otherwise it will be updated. The response
        /// will be a map between ticker -> updated.
        async fn upsert_markets(
            self: std::sync::Arc<Self>,
            request: tonic::Request<super::MsgUpsertMarkets>,
        ) -> std::result::Result<
            tonic::Response<super::MsgUpsertMarketsResponse>,
            tonic::Status,
        >;
    }
    /// Msg is the message service for the x/marketmap module.
    #[derive(Debug)]
    pub struct MsgServer<T: Msg> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Msg> MsgServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for MsgServer<T>
    where
        T: Msg,
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
                "/slinky.marketmap.v1.Msg/CreateMarkets" => {
                    #[allow(non_camel_case_types)]
                    struct CreateMarketsSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgCreateMarkets>
                    for CreateMarketsSvc<T> {
                        type Response = super::MsgCreateMarketsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgCreateMarkets>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Msg>::create_markets(inner, request).await
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
                        let method = CreateMarketsSvc(inner);
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
                "/slinky.marketmap.v1.Msg/UpdateMarkets" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateMarketsSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgUpdateMarkets>
                    for UpdateMarketsSvc<T> {
                        type Response = super::MsgUpdateMarketsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgUpdateMarkets>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Msg>::update_markets(inner, request).await
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
                        let method = UpdateMarketsSvc(inner);
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
                "/slinky.marketmap.v1.Msg/UpdateParams" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateParamsSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgParams>
                    for UpdateParamsSvc<T> {
                        type Response = super::MsgParamsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgParams>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Msg>::update_params(inner, request).await
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
                        let method = UpdateParamsSvc(inner);
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
                "/slinky.marketmap.v1.Msg/RemoveMarketAuthorities" => {
                    #[allow(non_camel_case_types)]
                    struct RemoveMarketAuthoritiesSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgRemoveMarketAuthorities>
                    for RemoveMarketAuthoritiesSvc<T> {
                        type Response = super::MsgRemoveMarketAuthoritiesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgRemoveMarketAuthorities>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Msg>::remove_market_authorities(inner, request).await
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
                        let method = RemoveMarketAuthoritiesSvc(inner);
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
                "/slinky.marketmap.v1.Msg/UpsertMarkets" => {
                    #[allow(non_camel_case_types)]
                    struct UpsertMarketsSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgUpsertMarkets>
                    for UpsertMarketsSvc<T> {
                        type Response = super::MsgUpsertMarketsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgUpsertMarkets>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as Msg>::upsert_markets(inner, request).await
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
                        let method = UpsertMarketsSvc(inner);
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
    impl<T: Msg> Clone for MsgServer<T> {
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
    impl<T: Msg> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Msg> tonic::server::NamedService for MsgServer<T> {
        const NAME: &'static str = "slinky.marketmap.v1.Msg";
    }
}
