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
use std::fmt::Display;

use serde::de::DeserializeOwned;

#[cfg(feature = "tests")]
pub mod tests;

/// The error that is returned if reading a config from the environment fails.
#[derive(Clone, Debug)]
pub struct Error {
    inner: figment::Error,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed reading config from process environment")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl From<figment::Error> for Error {
    fn from(inner: figment::Error) -> Self {
        Self {
            inner,
        }
    }
}

/// Utility function to get a config without having to import the `Config` trait.
///
/// # Errors
///
/// Returns the same error as `<T as Config>::get`.
pub fn get<T: Config>() -> Result<T, Error> {
    T::get()
}

/// A utility trait for easily creating a config from the environment.
///
/// Works for types allowing serde deserialization. Environment variables
/// are expected to have the form `<Config::PREFIX><fieldname>`. It is recommended
/// to set the prefix with a trailing underscore `PREFIX = MY_CONFIG_`
/// for readability.
///
/// # Usage
///
/// ```no_run
/// # use std::net::SocketAddr;
/// # use astria_config as config;
/// # use serde::Deserialize;
/// # use config::Error;
///
/// #[derive(Clone, Debug, Deserialize)]
/// pub struct MyConfig {
///     pub log: String,
///     pub api_listen_addr: SocketAddr,
/// }
///
/// impl config::Config for MyConfig {
///     const PREFIX: &'static str = "MY_SERVICE_";
/// }
///
/// let config: MyConfig = config::get()?;
/// # Ok::<_, Error>(())
/// ```
pub trait Config: core::fmt::Debug + DeserializeOwned {
    const PREFIX: &'static str;

    /// Creates `Self` by reading its fields from the environment.
    ///
    /// # Errors
    /// Returns an error if a config field could not be read from the environment.
    fn get() -> Result<Self, Error> {
        Ok(Self::get_with_prefix(Self::PREFIX, _internal::Internal)?)
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
