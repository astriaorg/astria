//! Astria's composer submits ethereum rollup transactions to astria's sequencer.
//!
//! At the moment composer can read from one ethereum rollup only, and submits each ethereum
//! as one sequencer transaction. At the moment it genrated a random sequencer account for
//! each submission.
//!
//! [`Composer`] is configured using a [`Config`] and started with [`Composer::run_until_stopped`].
//!
//!
//! # Examples
//!
//! ```no_run
//! # use astria_composer::{
//! #     Composer,
//! #     config,
//! #     telemetry,
//! };
//! # use tracing::info;
//! # tokio_test::block_on(async {
//! let cfg = config::get().expect("failed to read configuration");
//! let cfg_ser = serde_json::to_string(&cfg)
//!     .expect("the json serializer should never fail when serializing to a string");
//! eprintln!("config:\n{cfg_ser}");
//!
//! telemetry::init(std::io::stdout, &cfg.log).expect("failed to initialize tracing");
//!
//! info!(config = cfg_ser, "initializing composer",);
//!
//! let _composer = Composer::from_config(&cfg)
//!     .await
//!     .expect("failed creating composer")
//!     .run_until_stopped()
//!     .await;
//! # })
//! ```

pub(crate) mod api;
mod composer;
pub mod config;
pub(crate) mod searcher;
pub mod telemetry;

pub use composer::Composer;
pub use config::Config;
