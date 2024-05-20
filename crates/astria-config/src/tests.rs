//! Test functions to ensure that a service's config and config examples are up to date.
//!
//! # Examples
//!
//! ```no_run
//! use serde::{
//!     Deserialize,
//!     Serialize,
//! };
//!
//! #[derive(Clone, Debug, Serialize, Deserialize)]
//! pub struct MyConfig {
//!     pub log: String,
//!     pub api_listen_addr: std::net::SocketAddr,
//! }
//!
//! impl astria_config::Config for MyConfig {
//!     const PREFIX: &'static str = "MY_SERVICE_";
//! }
//!
//! const EXAMPLE_ENV: &str = r#"
//! MY_SERVICE_LOG="debug";
//! MY_SERVICE_API_LISTEN_ADDR="127.0.0.1:0"
//! "#;
//!
//! #[test]
//! fn example_env_config_is_up_to_date() {
//!     astria_config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
//! }
//! ```

use figment::Jail;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    Config,
    _internal,
};

static TEST_PREFIX: Lazy<String> = Lazy::new(|| {
    use names::{
        Generator,
        Name,
    };
    Generator::with_naming(Name::Numbered).next().unwrap()
});

fn populate_environment_from_example(jail: &mut Jail, unique_test_prefix: &str, example_env: &str) {
    static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
    static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());

    for line in example_env.lines() {
        if let Some((key, val)) = line.trim().split_once('=') {
            assert!(
                !(RE_END.is_match(key) || RE_START.is_match(val)),
                "env vars must not contain spaces in assignment\n{line}"
            );
            let prefixed_key = format!("{unique_test_prefix}_{key}");
            jail.set_env(prefixed_key, val);
        }
    }
}

/// Asserts that a config `C` can be created from a string holding env vars.
///
/// An environment string could, for example, be produced by the `include_str!`
/// macro with an dotenv file documenting the env vars a service takes as config.
///
/// # Panics
/// Panics if a config `C` could not be created from `example_env`.
#[track_caller]
pub fn example_env_config_is_up_to_date<C: Config>(example_env: &str) {
    let unique_test_prefix = Lazy::force(&TEST_PREFIX);
    let full_test_prefix = format!("{unique_test_prefix}_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, unique_test_prefix, example_env);
        C::get_with_prefix(&full_test_prefix, _internal::Internal)
            .unwrap_or_else(|error| panic!("failed to parse config: {error}"));
        Ok(())
    });
}
