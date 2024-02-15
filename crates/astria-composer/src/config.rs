use std::net::SocketAddr;

use secrecy::{
    zeroize::ZeroizeOnDrop,
    ExposeSecret as _,
    SecretString,
};
use serde::{
    Deserialize,
    Serialize,
    Serializer,
};

// this is a config, may have many boolean values
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, Serialize)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the API server
    pub api_listen_addr: SocketAddr,

    /// Address of the RPC server for the sequencer chain
    pub sequencer_url: String,

    /// A list of <chain_id>::<url> pairs
    pub rollups: String,

    /// Private key for the sequencer account used for signing transactions
    #[serde(serialize_with = "serialize_private_key")]
    pub private_key: SecretString,

    /// Sequencer block time in milliseconds
    #[serde(alias = "max_submit_interval_ms")]
    pub block_time_ms: u64,

    /// Max bytes to encode into a single sequencer `SignedTransaction`, not including signature,
    /// public key, nonce. This is the sum of the sizes of all the `SequenceAction`s
    pub max_bytes_per_bundle: usize,

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
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_COMPOSER_";
}

impl ZeroizeOnDrop for Config {}

fn serialize_private_key<S>(key: &SecretString, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::Error as _;
    let mut raw_key = key.expose_secret().clone().into_bytes();
    if let Some(sub_slice) = raw_key.get_mut(4..) {
        sub_slice.fill(b'#');
    }
    let sanitized_key = std::str::from_utf8(&raw_key)
        .map_err(|_| S::Error::custom("private key hex contained non-ascii characters"))?;
    s.serialize_str(sanitized_key)
}

#[cfg(test)]
mod tests {
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }
}
