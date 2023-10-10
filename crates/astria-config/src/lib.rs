use serde::{
    Deserialize,
    Serialize,
};

#[cfg(feature = "config-tests")]
mod config_tests;

#[cfg(feature = "attribute")]
pub use astria_config_attribute::astria_config;
#[cfg(feature = "config-tests")]
pub use config_tests::{
    config_test_suite_test_should_fail_with_bad_prefix,
    config_test_suite_test_should_populate_config_with_env_vars,
};

pub trait AstriaConfig<'a>: Serialize + Deserialize<'a> {
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
