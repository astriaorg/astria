//! Test functions to ensure that a service's config and config examples are up to date.
//!
//! # Examples
//!
//! ```rust,ignore
//! use astria_config::{example_env_config_is_up_to_date, config_should_reject_unknown_var};
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
//! const EXAMPLE_ENV: &str = include_str!("../local.env.example");
//!
//! #[test]
//! fn example_env_config_is_up_to_date() {
//!     config::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
//! }
//!
//! #[test]
//! #[should_panic]
//! fn config_should_reject_unknown_var() {
//!     config::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
//! }
//! ```

use figment::Jail;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    Config,
    _internal,
};

fn populate_environment_from_example(jail: &mut Jail, test_envar_prefix: &str, example_env: &str) {
    const RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
    const RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());

    for line in example_env.lines() {
        if let Some((key, val)) = line.trim().split_once('=') {
            assert!(
                !(RE_END.is_match(key) || RE_START.is_match(val)),
                "env vars must not contain spaces in assignment\n{line}"
            );
            let prefixed_key = format!("{test_envar_prefix}_{key}");
            dbg!(&prefixed_key);
            dbg!(&val);
            jail.set_env(prefixed_key, val);
        }
    }
}

#[track_caller]
pub fn example_env_config_is_up_to_date<'a, C: Config>(example_env: &str) {
    let test_prefix = format!("TESTTEST_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, "TESTTEST", example_env);
        C::get_with_prefix(test_prefix.as_str(), _internal::Internal).unwrap();
        Ok(())
    });
}

#[track_caller]
pub fn config_should_reject_unknown_var<'a, C: Config>(example_env: &str) {
    let test_prefix = format!("TESTTEST_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, "TESTTEST", example_env);
        let bad_prefix = format!("{}_FOOBAR", test_prefix);
        jail.set_env(bad_prefix, "BAZ");
        C::get_with_prefix(test_prefix.as_str(), _internal::Internal).unwrap();
        Ok(())
    });
}
