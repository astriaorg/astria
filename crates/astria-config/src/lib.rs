//! A trait to read a config from the environment.
//!
//! # Example
//! ```no_run
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
//!     pub api_listen_addr: std::net::SocketAddr,
//! }
//!
//! impl config::Config for MyConfig {
//!     const PREFIX: &'static str = "MY_SERVICE_";
//! }
//!
//! let config: MyConfig = config::get().unwrap();
//! ```
use serde::{
    de::DeserializeOwned,
    Serialize,
};

#[cfg(feature = "tests")]
mod tests;

#[cfg(feature = "tests")]
pub use tests::{
    config_should_reject_unknown_var,
    example_env_config_is_up_to_date,
};

/// Utility function to get a config without having to import the `Config` trait.
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
