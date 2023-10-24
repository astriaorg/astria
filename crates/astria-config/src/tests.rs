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
//! #[serde(deny_unknown_fields)]
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
//!
//! #[test]
//! #[should_panic]
//! fn config_should_reject_unknown_var() {
//!     astria_config::tests::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
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
    const RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
    const RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());

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

#[track_caller]
pub fn example_env_config_is_up_to_date<'a, C: Config>(example_env: &str) {
    let unique_test_prefix = Lazy::force(&TEST_PREFIX);
    let full_test_prefix = format!("{unique_test_prefix}_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, unique_test_prefix, example_env);
        C::get_with_prefix(&full_test_prefix, _internal::Internal).unwrap();
        Ok(())
    });
}

#[track_caller]
pub fn config_should_reject_unknown_var<'a, C: Config>(example_env: &str) {
    let unique_test_prefix = Lazy::force(&TEST_PREFIX);
    let full_test_prefix = format!("{unique_test_prefix}_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, unique_test_prefix, example_env);
        let bad_var = format!("{full_test_prefix}_FOOBAR");
        jail.set_env(bad_var, "BAZ");
        C::get_with_prefix(&full_test_prefix, _internal::Internal).unwrap_err();
        Ok(())
    });
}
