#[cfg(feature = "config")]
mod config;
#[cfg(feature = "config-tests")]
mod config_tests;

#[cfg(feature = "derive")]
pub use astria_config_derive::astria_config;
#[cfg(feature = "config")]
pub use config::{
    get_config,
    AstriaConfig,
};
#[cfg(feature = "config-tests")]
pub use config_tests::{
    config_should_reject_unknown_var,
    example_env_config_is_up_to_date,
};
