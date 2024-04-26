use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
};

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    WrapErr,
};
use base64::{
    prelude::BASE64_STANDARD,
    Engine as _,
};
use serde::{
    Deserialize,
    Serialize,
};

// Allowed `struct_excessive_bools` because this is used as a container
// for deserialization. Making this a builder-pattern is not actionable.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
pub struct Config {
    pub cometbft_endpoint: String,
    pub sequencer_grpc_endpoint: String,
    pub celestia_app_grpc_endpoint: String,
    pub celestia_app_key_file: String,
    pub block_time: u64,
    pub relay_only_validator_key_blocks: bool,
    #[serde(default)]
    pub validator_key_file: String,
    // Would ideally be private; accessed via the public getter which converts this to a collection
    // of `RollupId`s.  Left public for integration tests.
    #[doc(hidden)]
    pub rollup_id_filter: String,
    // The socket address at which sequencer relayer will server healthz, readyz, and status calls.
    pub api_addr: String,
    pub log: String,
    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,
    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,
    /// Set to true to disable the metrics server
    pub no_metrics: bool,
    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
    /// Writes a human readable format to stdout instead of JSON formatted OTEL trace data.
    pub pretty_print: bool,
    /// The path to which relayer will write its state prior to submitting to Celestia.
    pub pre_submit_path: PathBuf,
    /// The path to which relayer will write its state after submitting to Celestia.
    pub post_submit_path: PathBuf,
}

impl Config {
    /// Returns the collection of deduplicated rollup IDs specified in the comma-separated string
    /// of base64-encoded IDs.
    ///
    /// # Errors
    /// Returns an error if any of the values cannot be parsed to a rollup ID.
    pub fn rollup_id_filter(&self) -> eyre::Result<IncludeRollup> {
        IncludeRollup::parse(&self.rollup_id_filter)
    }
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_RELAYER_";
}

/// A filter which can be used to determine whether a given rollup should have its data submitted
/// to Celestia or not.
///
/// It is constructed from the `ASTRIA_SEQUENCER_RELAYER_ROLLUP_ID_FILTER` env var.  A given rollup
/// should have its data submitted if it is listed in the env var, or if the env var is empty.
#[derive(Clone, Debug)]
pub struct IncludeRollup(Arc<HashSet<RollupId>>);

impl IncludeRollup {
    /// Returns `true` if the given rollup should have its data included for submission.
    #[must_use]
    pub fn should_include(&self, rollup_id: &RollupId) -> bool {
        self.0.is_empty() || self.0.contains(rollup_id)
    }

    fn parse(input: &str) -> eyre::Result<Self> {
        let rollup_ids = input
            .split(',')
            .filter(|base64_encoded_id| !base64_encoded_id.is_empty())
            .map(|base64_encoded_id| {
                BASE64_STANDARD
                    .decode(base64_encoded_id.trim())
                    .wrap_err_with(|| {
                        format!(
                            "failed to base64-decode rollup id `{base64_encoded_id}` in \
                             configured rollup_id_filter"
                        )
                    })
                    .and_then(|raw_id| {
                        RollupId::try_from_slice(&raw_id).wrap_err_with(|| {
                            format!(
                                "failed to parse `{base64_encoded_id}` as a rollup id in \
                                 configured rollup_id_filter"
                            )
                        })
                    })
            })
            .collect::<eyre::Result<HashSet<RollupId>>>()?;
        Ok(Self(Arc::new(rollup_ids)))
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::RollupId;
    use itertools::Itertools;

    use super::*;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn should_create_filter() {
        let rollup_ids: HashSet<_> = (0..10).map(|i| RollupId::new([i; 32])).collect();

        // Normal form: "aaa,bbb,ccc".
        let input = rollup_ids.iter().join(",").to_string();
        assert_eq!(*IncludeRollup::parse(&input).unwrap().0, rollup_ids);

        // With trailing comma: "aaa,bbb,ccc,".
        let input = format!("{},", rollup_ids.iter().join(","));
        assert_eq!(*IncludeRollup::parse(&input).unwrap().0, rollup_ids);

        // With extra commas: "aaa,,bbb,,ccc,,".
        let input = format!("{},,", rollup_ids.iter().join(",,"));
        assert_eq!(*IncludeRollup::parse(&input).unwrap().0, rollup_ids);

        // With spaces after commas: "aaa, bbb, ccc".
        let input = rollup_ids.iter().join(", ").to_string();
        assert_eq!(*IncludeRollup::parse(&input).unwrap().0, rollup_ids);

        // With spaces before and after commas: "aaa , bbb , ccc".
        let input = rollup_ids.iter().join(" , ").to_string();
        assert_eq!(*IncludeRollup::parse(&input).unwrap().0, rollup_ids);

        // Single entry: "aaa".
        let single_id = RollupId::new([100; 32]);
        let input = single_id.to_string();
        assert_eq!(
            *IncludeRollup::parse(&input).unwrap().0,
            std::iter::once(single_id).collect(),
            "{input}"
        );

        // No entries: "".
        assert!(IncludeRollup::parse("").unwrap().0.is_empty());
    }

    #[test]
    fn should_fail_to_create_filter_from_bad_input() {
        // Invalid base64 encoding.
        let input = "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg!";
        let _ = IncludeRollup::parse(input).unwrap_err();

        // Invalid decoded length (31 bytes).
        let input = BASE64_STANDARD.encode([0; 31]);
        let _ = IncludeRollup::parse(&input).unwrap_err();
    }
}
