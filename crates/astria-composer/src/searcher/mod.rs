use std::{
    net::SocketAddr,
    sync::Arc,
};

use axum::{
    routing::get,
    Router,
};
use thiserror::Error;
use tracing::{
    debug,
    error,
};

use self::api::healthz;
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

    pub async fn run(self) -> Result<(), SearcherError> {
        let api_handle = tokio::spawn(async move {
            // TODO: this moves self, it shouldn't
            self.run_api().await
        });

        tokio::select! {
            o = api_handle => {
                match o {
                    Ok(_) => {
                        debug!("api server exited successfully");
                    }
                    Err(e) => {
                        error!("api server exited unexpectedly: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn run_api(&self) -> Result<(), SearcherError> {
        let api_router = Router::new()
            .route("/healthz", get(healthz))
            .with_state(self.state.clone());

        match axum::Server::bind(&self.api_url)
            .serve(api_router.into_make_service())
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => Err(SearcherError::ApiError(e)),
        }
    }
}
