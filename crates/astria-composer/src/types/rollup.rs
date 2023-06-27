use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainIdError {
    #[error("invalid chain id: {0}")]
    InvalidChainId(String),
}

/// Chain ID for a rollup
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ChainId(String);

impl FromStr for ChainId {
    type Err = ChainIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}
