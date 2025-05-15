#![cfg_attr(
    feature = "benchmark",
    allow(
        dead_code,
        reason = "we use the same functionality for tests and benchmarks, but benchmarks only \
                  need a subset of this"
    )
)]

pub(crate) mod accounts;
pub(crate) mod address;
pub(crate) mod app;
pub(crate) mod assets;
pub(crate) mod authority;
#[cfg(feature = "benchmark")]
pub(crate) mod benchmark_utils;
pub(crate) mod bridge;
mod build_info;
pub(crate) mod checked_actions;
pub(crate) mod checked_transaction;
pub(crate) mod component;
pub mod config;
pub(crate) mod fees;
pub(crate) mod grpc;
pub(crate) mod ibc;
mod mempool;
pub(crate) mod metrics;
pub(crate) mod oracles;
pub(crate) mod proposal;
mod sequencer;
pub(crate) mod service;
pub(crate) mod storage;
#[cfg(any(test, feature = "benchmark"))]
pub(crate) mod test_utils;
pub(crate) mod upgrades;
mod utils;

pub use build_info::BUILD_INFO;
pub use config::Config;
pub use metrics::Metrics;
pub use sequencer::Sequencer;
pub use telemetry;
