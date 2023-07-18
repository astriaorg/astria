use tokio::task::JoinError;

use super::{
    bundler,
    collector,
    executor,
};

#[derive(Debug, thiserror::Error)]
pub enum ComposerError {
    #[error("task error")]
    TaskError(#[source] JoinError),
    #[error("api error")]
    ApiError(#[source] hyper::Error),
    #[error("collector error")]
    CollectorError(#[source] collector::Error),
    #[error("bundler error")]
    BundlerError(#[source] bundler::Error),
    #[error("executor error")]
    ExecutorError(#[source] executor::Error),
    #[error("sequencer client init failed")]
    SequencerClientInit,
}
