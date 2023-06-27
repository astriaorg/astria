use std::{
    net::SocketAddr,
    sync::Arc,
};

use thiserror::Error;
use tokio::task::JoinSet;
use tracing::{
    error,
    info,
};

use self::api::ApiError;
use crate::config::searcher::{
    Config,
    ConfigError,
};

mod api;

#[derive(Debug, Error)]
pub enum SearcherError {
    #[error("invalid config")]
    InvalidConfig(#[from] ConfigError),
    #[error("api error")]
    ApiError(#[from] ApiError),
}

#[derive(Debug, Clone)]
pub(crate) struct State(Arc<()>);

pub struct Searcher {
    state: State,
    api_url: SocketAddr,
}

impl Searcher {
    pub fn new(config: Config) -> Result<Self, SearcherError> {
        // configure rollup tx collector
        // configure rollup tx bundler
        // configure rollup tx executor

        // init searcher state
        let state = State(Arc::new(()));

        // parse api url from config and init api router
        let api_url = config
            .api_url
            .parse::<SocketAddr>()
            .map_err(|e| SearcherError::InvalidConfig(e.into()))?;

        Ok(Self {
            state,
            api_url,
        })
    }

    pub fn api_url(&self) -> SocketAddr {
        self.api_url
    }

    /// Runs the Searcher and blocks until all subtasks have exited.
    pub async fn run(self) -> Result<(), SearcherError> {
        // Start engine.
        if let Ok(mut set) = self.run_subtasks().await {
            while let Some(res) = set.join_next().await {
                info!("res: {:?}", res);
                // TODO: bubble subtask errors up to main if they cant be handled?
            }
        }

        Ok(())
    }

    /// Runs the Searcher's subtasks and returns a handle to the JoinSet containing:
    /// - api server
    /// TODO:
    /// - rollup tx collector
    /// - rollup tx bundler
    /// - rollup tx executor
    async fn run_subtasks(&self) -> Result<JoinSet<()>, SearcherError> {
        // JoinSet because we want to run all subtasks in parallel
        let mut set = JoinSet::new();

        // TODO: doesn't compile if i borrow these from self inside the task instead of cloning
        let api_url = self.api_url.clone();
        let state = self.state.clone();

        set.spawn(async move {
            info!("starting api server");
            // TODO: if i borrow these from self it moves self, it shouldn't be necessary to clone
            // it should be:
            // match run_api(&self.api_url, &self.state).await {
            match api::run(&api_url, &state).await {
                Ok(()) => (),
                Err(e) => {
                    error!("api server exited unexpectedly: {:?}", e);
                    // TODO: handle api server error and if cant, return as searcher error?
                }
            }
        });

        Ok(set)
    }
}

mod tests {
    #[test]
    fn new_from_valid_config() {
        todo!("successful init from valid config")
    }

    #[test]
    fn new_from_invalid_config_fails() {
        todo!("failed init from invalid config")
    }
}
