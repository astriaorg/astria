#[cfg(feature = "config")]
mod config;
#[cfg(feature = "config-tests")]
mod config_tests;

#[cfg(feature = "config")]
pub use config::{get_config, AstriaConfig};

#[cfg(feature = "config-tests")]
pub use config_tests::{
    config_test_suite_test_should_populate_config_with_env_vars,
    config_test_suite_test_should_fail_with_bad_prefix,
};
#[cfg(feature="derive")]
pub use astria_config_derive::astria_config;
