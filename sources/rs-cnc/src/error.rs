use std::fmt;
use std::result::Result as StdResult;

use reqwest::Error as ReqwestError;
use thiserror;

/// A specialized `Result` type for rs-cnc.
pub type Result<T> = StdResult<T, Error>;


#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error occurred with the http client
    HttpClient(String),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::HttpClient(error) => write!(fmt, "http client error {}", error),
        }
    }
}

impl From<ReqwestError> for Error {
    fn from(e: ReqwestError) -> Self {
        Self::HttpClient(e.to_string())
    }
}
