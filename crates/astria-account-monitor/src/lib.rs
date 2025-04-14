//! Astria's account monitor tracks account information on Astria's sequencer.
//!
//! The account monitor periodically polls the Astria Shared Sequencer to retrieve and track
//! information about accounts. It monitors two types of accounts:
//!
//! 1. Regular accounts: Tracks nonces and balances
//! 2. Bridge accounts: Tracks transaction history and heights
//!
//! Account data is exposed through metrics for monitoring and alerting purposes.
//! The polling interval is configurable, and the service continues until it receives
//! a shutdown signal.
//!
//! [`AccountMonitor`] is configured using a [`Config`] and started with
//! [`AccountMonitor::run_until_stopped`].
//!
//!
//! # Examples
//!
//! ```no_run
//! # use astria_account_monitor::{
//! #     AccountMonitor,
//! #     Config,
//! #     telemetry,
//! # };
//! # use tracing::info;
//! # tokio_test::block_on(async {
//! let cfg: Config = config::get().expect("failed to read configuration");
//! let cfg_ser = serde_json::to_string(&cfg)
//!     .expect("the json serializer should never fail when serializing to a string");
//! eprintln!("config:\n{cfg_ser}");
//!
//! let (metrics, _telemetry_guard) = telemetry::configure()
//!     .set_filter_directives(&cfg.log)
//!     .try_init(&cfg)
//!     .expect("failed to setup telemetry");
//! info!(config = cfg_ser, "initializing account monitor",);
//!
//! let monitor = AccountMonitor::from_config(&cfg, metrics)
//!     .await
//!     .expect("failed creating account monitor");
//! let _ = monitor.run_until_stopped().await;
//! # })
//! ```

pub mod account_monitor;
mod build_info;
pub mod config;
pub(crate) mod metrics;
pub use account_monitor::AccountMonitor;
pub use build_info::BUILD_INFO;
use config::Config;
pub use metrics::Metrics;
