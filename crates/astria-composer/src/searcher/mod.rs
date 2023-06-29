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
    #[error("task error")]
    TaskError(#[from] JoinError),
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

        let api_task = tokio::spawn(api::run(api_url, state.clone()));
        tokio::select! {
            o = api_task => {
                match o {
                    Ok(task_result) => {
                        match task_result {
                            Ok(()) => report_exit("api server", Ok(())),
                // TODO: maybe handle api server failing and only return SearcherError::ApiError if can't?
                            Err(e) => report_exit("api server", Err(SearcherError::ApiError(e))),
                        }
                    }
                    Err(e) => {
                        report_exit("api server", Err(SearcherError::TaskError(e)))
                    }
                }
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
            .map_err(|e| SearcherError::ApiError(e.into()))?)
    }
}

fn report_exit(task_name: &str, outcome: Result<(), SearcherError>) {
    match outcome {
        Ok(()) => tracing::info!(task = task_name, "task exited successfully"),
        Err(e) => match e {
            SearcherError::TaskError(join_err) => {
                tracing::error!(task = task_name, error.msg = %join_err, error.cause = ?join_err, "task failed to complete")
            }
            service_err => {
                tracing::error!(task = task_name, error.msg = %service_err, error.cause = ?service_err, "task exited with error")
            }
        },
    }
}

mod tests {
    use super::Searcher;
    use crate::{
        config::searcher::{
            Config,
            ConfigError,
        },
        searcher::SearcherError,
    };

    #[test]
    fn new_from_valid_config() {
        let cfg = Config::default();
        let searcher = Searcher::new(cfg);
        assert!(searcher.is_ok())
    }
}
