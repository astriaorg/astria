//! A trait to read a config from the environment.
//!
//! # Usage
//!
//! Add this to your `Cargo.toml` (assuming your crate is in sibling directory of `astria-config`):
//! ```toml
//! [dependencies]
//! config = { package = "astria-config", path = "../astria-config" }
//! ````
//! The `Config` trait can then be implemented for your config type and constructed from the
//! environment either through the `Config::get` trait method or the `get` utility function shown in
//! the example below.
//! ```
//! use std::net::SocketAddr;
//!
//! use astria_config as config;
//! use serde::{
//!     Deserialize,
//!     Serialize,
//! };
//!
//! #[derive(Clone, Debug, Serialize, Deserialize)]
//! #[serde(deny_unknown_fields)]
//! pub struct MyConfig {
//!     pub log: String,
//!     pub api_listen_addr: SocketAddr,
//! }
//!
//! impl config::Config for MyConfig {
//!     const PREFIX: &'static str = "MY_SERVICE_";
//! }
//! # std::env::set_var("MY_SERVICE_LOG", "debug");
//! # std::env::set_var("MY_SERVICE_API_LISTEN_ADDR", "127.0.0.1:8080");
//! let config: MyConfig = config::get().expect("all config options should be set and valid");
//! assert_eq!(config.log, "debug");
//! assert_eq!(
//!     config.api_listen_addr,
//!     SocketAddr::from(([127, 0, 0, 1], 8080))
//! );
//! ```
//!
//! ## Crate feature flags
//!
//! + `tests`: gives access to test functions that to ensure that a crate's config is up-to-date and
//!   in sync with its example. See [`tests`] for how to use them.
use serde::{
    de::DeserializeOwned,
    Serialize,
};

#[cfg(feature = "tests")]
pub mod tests;

// Utility function to get a config without having to import the `Config` trait.
pub fn get<T: Config>() -> Result<T, figment::Error> {
    T::get()
}

pub trait Config: Serialize + DeserializeOwned {
    const PREFIX: &'static str;

    fn get() -> Result<Self, figment::Error> {
        Self::get_with_prefix(Self::PREFIX, _internal::Internal)
    }

    #[doc(hidden)]
    fn get_with_prefix(
        prefix: &str,
        _internal: _internal::Internal,
    ) -> Result<Self, figment::Error> {
        use figment::{
            providers::Env as FigmentEnv,
            Figment,
        };
        Figment::new()
            .merge(FigmentEnv::prefixed("RUST_").split("_").only(&["log"]))
            .merge(FigmentEnv::prefixed(prefix))
            .extract()
    }
}

mod _internal {
    pub struct Internal;
}
