use figment::Jail;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::AstriaConfig;

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

/// Test suite for testing configs according to the Astria spec
/// # Example for running the test suite
///
/// ```rust,ignore
/// mod test {
///     use astria_config::{
///         config_test_suite_test_should_fail_with_bad_prefix,
///         config_test_suite_test_should_populate_config_with_env_vars,
///     };
///
///     use crate::Config;
///
///     const EXAMPLE_ENV: &str = include_str!("../local.env.example");
///
///     #[test]
///     fn test_config_passing() {
///         config_test_suite_test_should_populate_config_with_env_vars::<Config>(EXAMPLE_ENV);
///     }
///
///     #[test]
///     #[should_panic]
///     fn test_config_failing() {
///         config_test_suite_test_should_fail_with_bad_prefix::<Config>(EXAMPLE_ENV);
///     }
/// }
/// ```

pub fn config_test_suite_test_should_populate_config_with_env_vars<'a, C>(example_env: &str)
where
    C: AstriaConfig<'a>,
{
    let test_prefix = format!("TESTTEST_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, "TESTTEST", example_env);
        C::from_environment(test_prefix.as_str()).unwrap();
        Ok(())
    });
}

pub fn config_test_suite_test_should_fail_with_bad_prefix<'a, C>(example_env: &str)
where
    C: AstriaConfig<'a>,
{
    let test_prefix = format!("TESTTEST_{}", C::PREFIX);

    Jail::expect_with(|jail| {
        populate_environment_from_example(jail, "TESTTEST", example_env);
        let bad_prefix = format!("{}_FOOBAR", test_prefix);
        jail.set_env(bad_prefix, "BAZ");
        C::from_environment(test_prefix.as_str()).unwrap();
        Ok(())
    });
}
