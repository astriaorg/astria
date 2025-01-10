//! TODO: Add a description

mod auctioneer;
mod bid;
mod block;
mod build_info;
pub mod config;
pub(crate) mod metrics;
mod rollup_channel;
mod sequencer_channel;
mod sequencer_key;
mod streaming_utils;

use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
pub use auctioneer::Auctioneer;
pub use build_info::BUILD_INFO;
pub use config::Config;
pub use metrics::Metrics;
pub use telemetry;
use tokio::task::JoinError;

fn flatten_join_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
