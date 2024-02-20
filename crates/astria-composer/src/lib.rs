//! Astria's composer submits EVM rollup transactions to astria's sequencer.
//!
//! At the moment composer can read from Geth based EVM rollups only, and submits each EVM
//! transaction (regardless of where it's collected from) as one sequencer transaction.
//! The submission nonces are based on a custom generated private key specific to the composer.
//! Each sequencer transaction is submitted with a new nonce which is one more than the nonce
//! used in the previous submission.
//!
//! [`Composer`] is configured using a [`Config`] and started with [`Composer::run_until_stopped`].
//!
//!
//! # Examples
//!
//! ```no_run
//! # use astria_composer::{
//! #     Composer,
//! #     Config,
//! #     telemetry,
//! };
//! # use tracing::info;
//! # tokio_test::block_on(async {
//! let cfg: Config = config::get().expect("failed to read configuration");
//! let cfg_ser = serde_json::to_string(&cfg)
//!     .expect("the json serializer should never fail when serializing to a string");
//! eprintln!("config:\n{cfg_ser}");
//!
//! telemetry::configure()
//!     .filter_directives(&cfg.log)
//!     .try_init()
//!     .expect("failed to setup telemetry");
//! info!(config = cfg_ser, "initializing composer",);
//!
//! let _composer = Composer::from_config(&cfg)
//!     .expect("failed creating composer")
//!     .run_until_stopped()
//!     .await;
//! # })
//! ```

pub(crate) mod api;
mod build_info;
mod composer;
pub mod config;
pub(crate) mod searcher;

pub use build_info::BUILD_INFO;
pub use composer::Composer;
pub use config::Config;
pub use telemetry;
