use std::fmt::Display;

use astria_core::primitive::v1::asset;
use serde::{
    Deserialize,
    Serialize,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "This is used as a container for deserialization. Making this a builder-pattern is \
              not actionable"
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-bridge service.
pub struct Config {
    // The sequencer service grpc endpoint used to fetch pending nonce.
    pub sequencer_grpc_endpoint: String,
    // The cometbft rpc endpoint for submitting transactions to the sequencer.
    pub sequencer_cometbft_endpoint: String,
    // The chain id of the sequencer chain.
    pub sequencer_chain_id: String,
    // Setting this to `true` disables frost threshold signing, falling back to signing via the key
    // at `sequencer_key_path`
    pub no_frost_threshold_signing: bool,
    // The path to the private key used to sign transactions submitted to the sequencer.
    // Only used if `no_frost_threshold_signing` is true.
    pub sequencer_key_path: String,
    // The minimum number of frost participants required to sign a transaction.
    pub frost_min_signers: usize,
    // The path to the json-encoded frost public key package.
    pub frost_public_key_package_path: String,
    // The frost participant gRPC endpoints.
    pub frost_participant_endpoints: FrostParticipantEndpoints,
    // The fee asset denomination to use for the bridge account's transactions.
    pub fee_asset_denomination: asset::Denom,
    // The asset denomination being withdrawn from the rollup.
    pub rollup_asset_denomination: asset::denom::TracePrefixed,
    // The bridge address corresponding to the bridged rollup asset on the sequencer.
    pub sequencer_bridge_address: String,
    // Whether to use compat addresses for `Ics20Withdrawal`s.
    pub use_compat_address: bool,
    // The address of the AstriaWithdrawer contract on the evm rollup.
    pub ethereum_contract_address: String,
    // The rpc endpoint of the evm rollup.
    pub ethereum_rpc_endpoint: String,
    // The address prefix to use when constructing sequencer addresses using the signing key.
    pub sequencer_address_prefix: String,
    // The socket address at which the bridge service will server healthz, readyz, and status
    // calls.
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
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_BRIDGE_WITHDRAWER_";
}

/// A simple container for a list of parsed URIs/gRPC endpoints.
///
/// Only provides `FromStr` and `IntoIterator` implementations
/// to parse a list of comma-separated URIs and iterate over them.
#[derive(Debug, Clone, PartialEq)]
pub struct FrostParticipantEndpoints {
    inner: Vec<tonic::transport::Uri>,
}

impl FrostParticipantEndpoints {
    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }
}

impl IntoIterator for FrostParticipantEndpoints {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = tonic::transport::Uri;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl Display for FrostParticipantEndpoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use itertools::Itertools as _;
        // XXX: formatting a URI appends a '/' to the URI. So if
        // a parsed URI did not end on a '/', formatting it will add it.
        write!(f, "{}", self.inner.iter().format(","))
    }
}

impl std::str::FromStr for FrostParticipantEndpoints {
    type Err = astria_eyre::eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use astria_eyre::eyre::WrapErr as _;
        let clients = if s.is_empty() {
            Vec::new()
        } else {
            s.split(',')
                .map(|s| {
                    s.parse().wrap_err_with(|| {
                        format!("failed to parse participant frost endpoint `{s}` as URI")
                    })
                })
                .collect::<Result<_, _>>()?
        };
        Ok(Self {
            inner: clients,
        })
    }
}

impl<'de> Deserialize<'de> for FrostParticipantEndpoints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = std::borrow::Cow::<'_, str>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for FrostParticipantEndpoints {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Config,
        FrostParticipantEndpoints,
    };

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[track_caller]
    fn assert_parsed_frost_endpoints(input: &str) {
        let endpoints: FrostParticipantEndpoints = input.parse().unwrap();
        assert_eq!(input, &endpoints.to_string());
    }

    #[test]
    fn parse_frost_endpoints() {
        assert_parsed_frost_endpoints("");
        assert_parsed_frost_endpoints("https://foo.bar/");
        assert_parsed_frost_endpoints("https://foo.bar/,https://baz.qux/");
    }
}
