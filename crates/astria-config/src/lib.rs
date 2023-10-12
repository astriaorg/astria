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
//! let config: MyConfig = config::Config::get().unwrap();
//! ```

use serde::{
    de::DeserializeOwned,
    Serialize,
};

#[cfg(feature = "config-tests")]
mod config_tests;

#[cfg(feature = "attribute")]
pub use astria_config_attribute::astria_config;
#[cfg(feature = "config-tests")]
pub use config_tests::{
    config_should_reject_unknown_var,
    example_env_config_is_up_to_date,
};

pub trait Config: Serialize + DeserializeOwned {
    const PREFIX: &'static str;

    fn get() -> Result<Self, figment::Error> {
        Self::get_with_prefix(Self::PREFIX, _internal::Internal)
    }

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
