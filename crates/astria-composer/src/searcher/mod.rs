use std::{
    net::SocketAddr,
    sync::Arc,
};

use axum::{
    routing::get,
    Router,
};
use thiserror::Error;
use tokio::task::JoinError;

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
    ApiError(#[from] hyper::Error),
}

#[derive(Debug)]
pub(crate) struct State();

pub struct Searcher {
    state: Arc<State>,
    api_url: SocketAddr,
}

impl Searcher {
    pub fn new(config: Config) -> Result<Self, SearcherError> {
        // configure rollup tx collector
        // configure rollup tx bundler
        // configure rollup tx executor

        // init searcher state
        let state = Arc::new(State());

        // parse api url from config and init api router
        Ok(Self {
            state,
            api_url: config.api_url,
        })
    }

    pub fn api_url(&self) -> SocketAddr {
        self.api_url
    }

    /// Runs the Searcher and blocks until all subtasks have exited.
    /// - api server
    /// TODO:
    /// - rollup tx collector
    /// - rollup tx bundler
    /// - rollup tx executor
    pub async fn run(self) {
        let Self {
            state,
            api_url,
        } = self;

        let api_task = tokio::spawn(Self::run_api(api_url, state.clone()));
        tokio::select! {
            o = api_task => {
                // TODO: maybe handle api server failing and only return SearcherError::ApiError if can't?
                report_exit("api server", o);
            }
        }
    }

    async fn run_api(api_url: SocketAddr, state: Arc<State>) -> Result<(), SearcherError> {
        let api_router = Router::new()
            .route("/healthz", get(api::healthz))
            .with_state(state);

        Ok(axum::Server::bind(&api_url)
            .serve(api_router.into_make_service())
            .await
            .map_err(SearcherError::ApiError)?)
    }
}

fn report_exit(task_name: &str, outcome: Result<Result<(), SearcherError>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!(task = task_name, "task exited successfully"),
        Ok(Err(e)) => {
            tracing::error!(task = task_name, error.msg = %e, errir.cause = ?e, "task exited with error")
        }
        Err(e) => {
            tracing::error!(task = task_name, error.msg = %e, errir.cause = ?e, "task failed to complete")
        }
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
