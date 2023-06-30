use std::{
    net::SocketAddr,
    sync::Arc,
};

use tokio::task::JoinError;

use crate::config::searcher::{
    self as config,
    Config,
};

mod api;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid config")]
    InvalidConfig(#[from] config::Error),
    #[error("api error")]
    ApiError(#[from] hyper::Error),
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
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    /// Returns a `searcher::Error::InvalidConfig` if there is an error constructing `api_url` from
    /// the port specified in config.
    pub fn new(cfg: &Config) -> Result<Self, Error> {
        // configure rollup tx collector
        // configure rollup tx bundler
        // configure rollup tx executor

        // init searcher state
        let state = Arc::new(State());

        // parse api url from config
        let api_url = Config::api_url(cfg.api_port)?;

        Ok(Self {
            state,
            api_url,
        })
    }

    /// Runs the Searcher and blocks until all subtasks have exited.
    /// - api server
    /// TODO:
    /// - rollup tx collector
    /// - rollup tx bundler
    /// - rollup tx executor
    /// # Errors
    /// Returns a `searcher::Error` if the Searcher fails to start or if any of the subtasks fail
    /// and cannot be recovered.
    pub async fn run(self) {
        let Self {
            state,
            api_url,
        } = self;

        let api_task = tokio::spawn(api::run(api_url, state.clone()));
        tokio::select! {
            o = api_task => {
                match o {
                    Ok(task_result) => report_exit("api server", task_result.map_err(Error::ApiError)),
                    Err(e) => report_exit("api server", Err(Error::TaskError(e))),

                }
            }
        }
    }
}

fn report_exit(task_name: &str, outcome: Result<(), Error>) {
    match outcome {
        Ok(()) => tracing::info!(task = task_name, "task exited successfully"),
        Err(e) => match e {
            Error::TaskError(join_err) => {
                tracing::error!(task = task_name, error.msg = %join_err, error.cause = ?join_err, "task failed to complete");
            }
            service_err => {
                tracing::error!(task = task_name, error.msg = %service_err, error.cause = ?service_err, "task exited with error");
            }
        },
    }
}

mod tests {
    use crate::{
        config::searcher::Config,
        searcher::Searcher,
    };

    #[test]
    fn new_from_valid_config() {
        let cfg = Config::default();
        let searcher = Searcher::new(&cfg);
        assert!(searcher.is_ok());
    }
}
