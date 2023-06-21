use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

use crate::types::ChainId;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid config")]
    InvalidConfig(),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub sequencer_endpoint: String,
    pub rollup_chain_ids: Vec<ChainId>,
}

impl Default for Config {
    fn default() -> Self {
        unimplemented!()
    }
}
