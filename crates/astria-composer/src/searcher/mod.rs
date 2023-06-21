use thiserror::Error;

use crate::config::searcher::{
    Config,
    ConfigError,
};

#[derive(Debug, Error)]
pub enum SearcherError {
    #[error("invalid config")]
    InvalidConfig(#[from] ConfigError),
}

pub struct Searcher {}

impl Searcher {
    pub fn new(config: Config) -> Result<Self, SearcherError> {
        Ok(Self {})
    }

    pub async fn run(self) -> Result<(), SearcherError> {
        unimplemented!()
    }
}
