pub mod blob;
pub mod header;
pub mod state;

pub mod rpc_impl;

pub use rpc_impl::blob::Blob;
#[cfg(feature = "server")]
pub use rpc_impl::state::StateServer;
pub(crate) mod serde;
#[cfg(test)]
pub(crate) mod test_utils;

#[derive(Debug)]
pub struct DeserializationError {
    pub(crate) source: serde_json::Error,
    pub(crate) rpc: &'static str,
    pub(crate) deser_target: &'static str,
    pub(crate) raw_json: Box<serde_json::value::RawValue>,
}

impl DeserializationError {
    #[must_use]
    pub fn raw_json(&self) -> &serde_json::value::RawValue {
        &self.raw_json
    }

    #[must_use]
    pub fn source(&self) -> &serde_json::Error {
        &self.source
    }
}

impl std::fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to deserialize response from `{rpc}` as `{deser_target}`; see \
             error.raw_json() for server response",
            rpc = self.rpc,
            deser_target = self.deser_target,
        )
    }
}

impl std::error::Error for DeserializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
pub struct Error {
    inner: ErrorKind,
    rpc: &'static str,
}

impl Error {
    pub(crate) fn deserialization(e: DeserializationError, rpc: &'static str) -> Self {
        Self {
            inner: ErrorKind::Deserialization(e),
            rpc,
        }
    }

    pub(crate) fn rpc(e: jsonrpsee::core::Error, rpc: &'static str) -> Self {
        Self {
            inner: ErrorKind::Rpc(e),
            rpc,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "`{}` RPC failed", self.rpc)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner.source())
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Rpc(jsonrpsee::core::Error),
    Deserialization(DeserializationError),
}

impl ErrorKind {
    fn source(&self) -> &(dyn std::error::Error + 'static) {
        match self {
            Self::Rpc(e) => e,
            Self::Deserialization(e) => e,
        }
    }
}

/// A celestia JSON RPC client.
#[derive(Clone, Debug)]
pub struct Client {
    inner: jsonrpsee::http_client::HttpClient,
}

impl Client {
    /// Construct a celestia client using a predefined [`HttpClient`].
    pub fn from_jsonrpsee_client(inner: jsonrpsee::http_client::HttpClient) -> Self {
        Self {
            inner,
        }
    }

    /// Construct a celestia client using the builder pattern.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }
}

/// Builder for the celestia JSON RPC client.
///
/// Configurable options:
/// + bearer token (required)
/// + endpoint (required)
#[derive(Debug, Default)]
pub struct ClientBuilder {
    bearer_token: Option<String>,
    endpoint: Option<String>,
}

impl ClientBuilder {
    /// Return a celestia client builder with all fields initialized to `None`.
    fn new() -> Self {
        Self::default()
    }

    /// Consume the celestia client builder, returning a celestia client.
    ///
    /// # Errors
    /// This methid will return errors in the following scenarios:
    /// + if the bearer token is not set;
    /// + if the endpoint is not set;
    /// + if the bearer token contains invalid characters;
    ///   + this method has the same error conditions as [`HeaderValue::from_str`];
    /// + if building the underlying jsonrpc client failed;
    ///   + this method has the same error conditions as
    ///     [`jsonrpsee::http_client::HttpClientBuilder::build`].
    pub fn build(self) -> Result<Client, BuildError> {
        use jsonrpsee::http_client::{
            HeaderMap,
            HeaderValue,
            HttpClientBuilder,
        };
        let Self {
            bearer_token,
            endpoint,
        } = self;
        let Some(bearer_token) = bearer_token else {
            return Err(BuildError::missing_bearer_token());
        };
        let Some(endpoint) = endpoint else {
            return Err(BuildError::missing_endpoint());
        };
        let mut headers = HeaderMap::new();
        let bearer_token_value = HeaderValue::from_str(&format!("Bearer {bearer_token}"))
            .map_err(BuildError::invalid_bearer_token)?;
        headers.insert("Authorization", bearer_token_value);
        let inner = HttpClientBuilder::default()
            .set_headers(headers)
            .build(endpoint)
            .map_err(BuildError::jsonrpsee_builder)?;
        Ok(Client::from_jsonrpsee_client(inner))
    }

    /// Sets the bearer token for the JSON RPC http client.
    #[must_use]
    pub fn bearer_token(self, bearer_token: &str) -> Self {
        self.set_bearer_token(Some(bearer_token.to_string()))
    }

    /// Sets or unsets the bearer token for the JSON RPC http client.
    #[must_use]
    pub fn set_bearer_token(self, bearer_token: Option<String>) -> Self {
        Self {
            bearer_token,
            ..self
        }
    }

    /// Sets the endpoint that the client will connect to.
    ///
    /// Note that the string must be a valid URI with a port number.
    /// Otherwise `ClientBuilder::build` will fail.
    #[must_use]
    pub fn endpoint(self, endpoint: &str) -> Self {
        self.set_endpoint(Some(endpoint.to_string()))
    }

    /// Sets or unsets the endpoint that the client will connect to.
    ///
    /// Note that the string must be a valid URI with a port number.
    /// Otherwise `ClientBuilder::build` will fail.
    #[must_use]
    pub fn set_endpoint(self, endpoint: Option<String>) -> Self {
        Self {
            endpoint,
            ..self
        }
    }
}

/// The errors that can occur while configuring the celestia client.
#[derive(Debug)]
pub struct BuildError {
    inner: Errors,
}

impl BuildError {
    /// Utility function to construct a `BuildError` containing an `InvalidBearerToken`
    fn invalid_bearer_token(e: http::header::InvalidHeaderValue) -> Self {
        Self {
            inner: Errors::InvalidBearerToken(InvalidBearerToken {
                inner: e,
            }),
        }
    }

    /// Utility function to construct a `BuildError` containing a `jsonrpsee::core::Error`
    fn jsonrpsee_builder(e: jsonrpsee::core::Error) -> Self {
        Self {
            inner: Errors::JsonRpseeBuilder(JsonRpseeBuilder {
                inner: e,
            }),
        }
    }

    /// Utility function to construct a `BuildError` containing a `MissingBearerToken`
    fn missing_bearer_token() -> Self {
        Self {
            inner: Errors::MissingBearerToken(MissingBearerToken {
                _inner: (),
            }),
        }
    }

    /// Utility function to construct a `BuildError` containing a `MissingEndpoint`
    fn missing_endpoint() -> Self {
        Self {
            inner: Errors::MissingEndpoint(MissingEndpoint {
                _inner: (),
            }),
        }
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

/// All errors that are contained inside `BuildError`.
///
/// Used to delegate various trait impls.
#[derive(Debug)]
enum Errors {
    InvalidBearerToken(InvalidBearerToken),
    JsonRpseeBuilder(JsonRpseeBuilder),
    MissingBearerToken(MissingBearerToken),
    MissingEndpoint(MissingEndpoint),
}

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Errors::InvalidBearerToken(inner) => inner.fmt(f),
            Errors::JsonRpseeBuilder(inner) => inner.fmt(f),
            Errors::MissingBearerToken(inner) => inner.fmt(f),
            Errors::MissingEndpoint(inner) => inner.fmt(f),
        }
    }
}

impl std::error::Error for Errors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Errors::InvalidBearerToken(inner) => inner.source(),
            Errors::JsonRpseeBuilder(inner) => inner.source(),
            Errors::MissingBearerToken(inner) => inner.source(),
            Errors::MissingEndpoint(inner) => inner.source(),
        }
    }
}

/// If the bearer token contained non-ascii characters.
#[derive(Debug)]
struct InvalidBearerToken {
    inner: http::header::InvalidHeaderValue,
}

/// If building the inner jsonrpsee client failed.
///
/// This usually happens if there is a problem with the underlying transport.
#[derive(Debug)]
struct JsonRpseeBuilder {
    inner: jsonrpsee::core::Error,
}

/// If the bearer token was not set on the client builder.
#[derive(Debug)]
struct MissingBearerToken {
    _inner: (),
}

/// If the endpoint was not set on the client builder.
#[derive(Debug)]
struct MissingEndpoint {
    _inner: (),
}

impl std::fmt::Display for InvalidBearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = "bearer token contained invalid characters";
        f.write_str(msg)
    }
}

impl std::fmt::Display for JsonRpseeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = "failed constructing underlying http client";
        f.write_str(msg)
    }
}

impl std::fmt::Display for MissingBearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = "bearer token not set on client builder: all requests to the celestia Node API \
                   must have a bearer token";
        f.write_str(msg)
    }
}

impl std::fmt::Display for MissingEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = "missing endpoint; an endpoint must be provided so that the client knows where \
                   to connect";
        f.write_str(msg)
    }
}

impl std::error::Error for InvalidBearerToken {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl std::error::Error for JsonRpseeBuilder {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl std::error::Error for MissingBearerToken {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl std::error::Error for MissingEndpoint {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
