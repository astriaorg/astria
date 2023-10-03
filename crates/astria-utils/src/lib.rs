#[cfg(feature = "config")]
mod config;
#[cfg(feature = "config-tests")]
mod config_tests;

#[cfg(feature = "config")]
pub use config::AstriaConfig;
#[cfg(feature = "config-tests")]
pub use config_tests::{
    config_test_suite_failing,
    config_test_suite_passing,
};
