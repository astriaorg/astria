use std::{
    error::Error,
    fmt::{self, Display, Formatter, Result},
};

use http::header::InvalidHeaderValue;

#[derive(Debug)]
pub struct RpcError {
    inner: jsonrpsee::core::Error,
}

impl RpcError {
    pub(super) fn from_jsonrpsee(inner: jsonrpsee::core::Error) -> Self {
        Self { inner }
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("jsonrpsee call failed")
    }
}

impl std::error::Error for RpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

#[derive(Debug)]
pub enum BuildError {
    FieldNotSet(FieldNotSet),
    InvalidBearerToken(InvalidBearerToken),
    InnerClient(InnerClient),
}

impl BuildError {
    pub(super) fn field_not_set(field: &'static str) -> Self {
        Self::FieldNotSet(FieldNotSet { field })
    }

    pub(super) fn invalid_bearer_token(error: InvalidHeaderValue) -> Self {
        Self::InvalidBearerToken(InvalidBearerToken { inner: error })
    }

    pub(super) fn inner_client(error: jsonrpsee::core::Error) -> Self {
        Self::InnerClient(InnerClient { inner: error })
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldNotSet(e) => e.fmt(f),
            Self::InvalidBearerToken(e) => e.fmt(f),
            Self::InnerClient(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::FieldNotSet(e) => e.source(),
            BuildError::InvalidBearerToken(e) => e.source(),
            BuildError::InnerClient(e) => e.source(),
        }
    }
}

#[derive(Debug)]
pub struct FieldNotSet {
    pub(super) field: &'static str,
}

impl Display for FieldNotSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("field not set: `")?;
        f.write_str(self.field)?;
        f.write_str("`")?;
        Ok(())
    }
}

impl Error for FieldNotSet {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub struct InvalidBearerToken {
    pub(super) inner: InvalidHeaderValue,
}

impl Display for InvalidBearerToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("the provided bearer token was invalid")
    }
}

impl Error for InvalidBearerToken {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

#[derive(Debug)]
pub struct InnerClient {
    pub(super) inner: jsonrpsee::core::Error,
}

impl Display for InnerClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("failed constructing the inner json rpc client")
    }
}

impl Error for InnerClient {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}
