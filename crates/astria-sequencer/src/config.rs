use std::{
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};

use serde::{
    Deserialize,
    Serialize,
};
use url::Url;

#[expect(
    clippy::struct_excessive_bools,
    reason = "this is used as a container for deserialization. Making this a builder-pattern is \
              not actionable"
)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub abci_listen_url: AbciListenUrl,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
    /// The gRPC endpoint
    pub grpc_addr: String,
    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,
    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,
    /// Set to true to disable the metrics server
    pub no_metrics: bool,
    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
    /// The maximum number of transactions that can be parked in the mempool.
    pub mempool_parked_max_tx_count: usize,
    /// Disables streaming optimistic blocks over grpc.
    pub no_optimistic_blocks: bool,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_";
}

#[derive(Debug)]
pub enum AbciListenUrl {
    Tcp(SocketAddr),
    Unix(PathBuf),
}

impl std::fmt::Display for AbciListenUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbciListenUrl::Tcp(socket_addr) => write!(f, "tcp://{socket_addr}"),
            AbciListenUrl::Unix(path) => write!(f, "unix://{}", path.display()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AbciListenUrlParseError {
    #[error(
        "parsed input as a tcp address `{parsed}`, but could not turn it into a socket address"
    )]
    TcpButBadSocketAddr { parsed: Url, source: std::io::Error },
    #[error(
        "parsed input as a unix domain socket URL `{parsed}`, but could not turn it into a path"
    )]
    UnixButBadPath { parsed: Url },
    #[error(
        "parsed input as `{parsed}`, but scheme `scheme` is not suppported; supported schemes are \
         tcp, unix"
    )]
    UnsupportedScheme { parsed: Url, scheme: String },
    #[error("failed parsing input as URL")]
    Url {
        #[from]
        source: url::ParseError,
    },
}

impl FromStr for AbciListenUrl {
    type Err = AbciListenUrlParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let abci_url = Url::parse(s)?;

        match abci_url.scheme() {
            "tcp" => match abci_url.socket_addrs(|| None) {
                Ok(mut socket_addrs) => {
                    let socket_addr = socket_addrs.pop().expect(
                        "the url crate is guaranteed to return vec with exactly one element \
                         because it relies on std::net::ToSocketAddrs::to_socket_addr; if this is \
                         no longer the case there was a breaking change in the url crate",
                    );
                    Ok(Self::Tcp(socket_addr))
                }
                Err(source) => Err(Self::Err::TcpButBadSocketAddr {
                    parsed: abci_url,
                    source,
                }),
            },
            "unix" => {
                if let Ok(path) = abci_url.to_file_path() {
                    Ok(Self::Unix(path))
                } else {
                    Err(Self::Err::UnixButBadPath {
                        parsed: abci_url,
                    })
                }
            }
            // If more options are added here will also need to update the server startup
            // immediately below to support more than two protocols.
            other => Err(Self::Err::UnsupportedScheme {
                parsed: abci_url.clone(),
                scheme: other.to_string(),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for AbciListenUrl {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = std::borrow::Cow::<'_, str>::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for AbciListenUrl {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    use super::AbciListenUrl;

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn unix_input_is_parsed_as_abci_listen_url() {
        let expected = "/path/to/unix.sock";
        #[expect(
            clippy::match_wildcard_for_single_variants,
            reason = "intended to match all future variants because the test is only valid for a \
                      single variant"
        )]
        match format!("unix://{expected}")
            .parse::<AbciListenUrl>()
            .unwrap()
        {
            AbciListenUrl::Unix(actual) => {
                assert_eq!(AsRef::<std::path::Path>::as_ref(expected), actual.as_path(),);
            }
            other => panic!("expected AbciListenUrl::Unix, got {other:?}"),
        }
    }

    #[test]
    fn tcp_input_is_parsed_as_abci_listen_url() {
        let expected = "127.0.0.1:0";
        #[expect(
            clippy::match_wildcard_for_single_variants,
            reason = "intended to match all future variants because the test is only valid for a \
                      single variant"
        )]
        match format!("tcp://{expected}")
            .parse::<AbciListenUrl>()
            .unwrap()
        {
            AbciListenUrl::Tcp(actual) => {
                assert_eq!(expected, actual.to_string());
            }
            other => panic!("expected AbciListenUrl, got {other:?}"),
        }
    }

    #[test]
    fn tcp_listen_addr_format() {
        assert_eq!(
            "tcp://127.0.0.1:0",
            &AbciListenUrl::Tcp(([127, 0, 0, 1], 0).into()).to_string()
        );
    }

    #[test]
    fn unix_listen_addr_format() {
        assert_eq!(
            "unix:///path/to/unix.sock",
            &AbciListenUrl::Unix("/path/to/unix.sock".into()).to_string(),
        );
    }

    // NOTE: the only genuine new error variant is AbciListenUrl. Tests for other error paths are
    // not provided because they are fundamentally wrappers of url crate errors.
    #[test]
    fn http_is_not_valid_abci_listen_scheme() {
        match "http://astria.org".parse::<AbciListenUrl>().unwrap_err() {
            super::AbciListenUrlParseError::UnsupportedScheme {
                scheme, ..
            } => assert_eq!("http", scheme),
            other => panic!("expected AbciListenUrlParseError::UnsupportedScheme, got `{other:?}`"),
        }
    }
}
