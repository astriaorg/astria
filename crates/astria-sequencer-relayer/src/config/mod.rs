use clap::Parser as _;
use figment::{
    providers::{
        Env,
        Serialized,
    },
    Figment,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

mod cli;

/// Max time in ms to wait for a block to finalize after it is received from the sequencer.
pub const MAX_RELAYER_QUEUE_TIME_MS: u64 = 2 * DEFAULT_BLOCK_TIME_MS;
/// Default block time in ms for a sequencer block.
pub const DEFAULT_BLOCK_TIME_MS: u64 = 1000;

const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26658";
const DEFAULT_SEQUENCER_ENDPOINT: &str = "http://localhost:26657";
const DEFAULT_VALIDATOR_KEY_FILE: &str = ".cometbft/config/priv_validator_key.json";

const DEFAULT_RPC_LISTEN_PORT: u16 = 2450;
const DEFAULT_GOSSIP_PORT: u16 = 33900;
const DEFAULT_LOG_DIRECTIVE: &str = "info";

/// Utility function to read the application's config in one go.
///
/// This includes parsing the command line args, getting all environment variables.
/// This function will short circuit binary execution when `--help` or `--version`
/// is provided, or if the command line arguments could not be read.
///
/// # Errors
///
/// An error is returned if the config could not be read.
pub fn get() -> Result<Config, figment::Error> {
    let cli_config = cli::Args::parse();
    // We generate `cmd` after making sure the command parses successfully
    // to access the binary name and version. It is not possible to get this
    // information through the parsed type itself.
    let cmd = <cli::Args as clap::CommandFactory>::command();
    eprintln!(
        "running {name}:{version}",
        name = cmd.get_name(),
        version = cmd.get_version().unwrap_or("<no-version-set>"),
    );
    Config::with_cli(cli_config)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub gas_limit: u64,
    pub disable_writing: bool,
    /// The relayer's `sequencer_poll_period` is set to the sequencer block time.
    pub sequencer_block_time_ms: u64,
    pub validator_key_file: String,
    pub rpc_port: u16,
    pub p2p_port: u16,
    #[serde(deserialize_with = "bootnodes_deserialize")]
    pub bootnodes: Option<Vec<String>>,
    pub libp2p_private_key: Option<String>,
    pub log: String,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    fn with_cli(cli_config: cli::Args) -> Result<Config, figment::Error> {
        // Extract RUST_LOG=<filter-directives>
        let rust_log = Env::prefixed("RUST_").split("_").only(&["log"]);
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(rust_log)
            .merge(Env::prefixed("ASTRIA_SEQUENCER_RELAYER_"))
            .merge(Serialized::defaults(cli_config))
            .extract()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            celestia_endpoint: DEFAULT_CELESTIA_ENDPOINT.into(),
            celestia_bearer_token: String::new(),
            sequencer_endpoint: DEFAULT_SEQUENCER_ENDPOINT.into(),
            gas_limit: crate::data_availability::DEFAULT_PFD_GAS_LIMIT,
            disable_writing: false,
            sequencer_block_time_ms: DEFAULT_BLOCK_TIME_MS,
            validator_key_file: DEFAULT_VALIDATOR_KEY_FILE.into(),
            rpc_port: DEFAULT_RPC_LISTEN_PORT,
            p2p_port: DEFAULT_GOSSIP_PORT,
            bootnodes: None,
            libp2p_private_key: None,
            log: DEFAULT_LOG_DIRECTIVE.into(),
        }
    }
}

fn bootnodes_deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_bootnodes: Option<String> = Option::deserialize(deserializer)?;
    if maybe_bootnodes.is_none() {
        return Ok(None);
    }
    Ok(Some(
        maybe_bootnodes
            .unwrap()
            .split(',')
            .map(|item| item.to_owned())
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use figment::Jail;

    use super::{
        cli,
        Config,
    };

    const NO_CLI_ARGS: &str = "astria-sequencer-relayer";
    const ALL_CLI_ARGS: &str = r#"
astria-sequencer-relayer
    --celestia-endpoint http://celestia.cli
    --celestia-bearer-token clibearertoken
    --sequencer-endpoint http://sequencer.cli
    --gas-limit 9999
    --disable-writing
    --block-time 9999
    --validator-key-file /cli/key
    --rpc-port 9999
    --p2p-port 9999
    --bootnodes /cli/bootnode1,/cli/bootnode2
    --libp2p-private-key libp2p.key
    --log cli=warn
"#;

    fn make_args(args: &str) -> Result<cli::Args, clap::Error> {
        cli::Args::try_parse_from(str::split_ascii_whitespace(args))
    }

    fn set_all_env(jail: &mut Jail) {
        jail.set_env(
            "ASTRIA_SEQUENCER_RELAYER_SEQUENCER_ENDPOINT",
            "http://sequencer.env",
        );
        jail.set_env(
            "ASTRIA_SEQUENCER_RELAYER_CELESTIA_ENDPOINT",
            "http://celestia.env",
        );
        jail.set_env(
            "ASTRIA_SEQUENCER_RELAYER_CELESTIA_BEARER_TOKEN",
            "envbearertoken",
        );
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_GAS_LIMIT", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_DISABLE_WRITING", true);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_BLOCK_TIME", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_VALIDATOR_KEY_FILE", "/env/key");
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_RPC_PORT", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_P2P_PORT", 5555);
        jail.set_env(
            "ASTRIA_SEQUENCER_RELAYER_BOOTNODES",
            "/cli/bootnode3,/cli/bootnode4",
        );
        jail.set_env(
            "ASTRIA_SEQUENCER_RELAYER_LIBP2P_PRIVATE_KEY",
            "envlibp2p.key",
        );
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_LOG", "env=debug");
    }

    #[test]
    fn cli_overrides_all() {
        Jail::expect_with(|jail| {
            set_all_env(jail);
            jail.set_env("ASTRIA_SEQUENCER_RELAYER_DISABLE_WRITING", false);
            let cli_args = make_args(ALL_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                sequencer_endpoint: "http://sequencer.cli".into(),
                celestia_endpoint: "http://celestia.cli".into(),
                celestia_bearer_token: "clibearertoken".into(),
                gas_limit: 9999,
                disable_writing: true,
                sequencer_block_time_ms: 9999,
                validator_key_file: "/cli/key".into(),
                rpc_port: 9999,
                p2p_port: 9999,
                bootnodes: Some(vec![
                    "/cli/bootnode1".to_string(),
                    "/cli/bootnode2".to_string(),
                ]),
                libp2p_private_key: Some("libp2p.key".to_string()),
                log: "cli=warn".into(),
            };
            assert_eq!(expected, actual);
            Ok(())
        })
    }

    #[test]
    fn env_overrides_default() {
        Jail::expect_with(|jail| {
            set_all_env(jail);
            let cli_args = make_args(NO_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                sequencer_endpoint: "http://sequencer.env".into(),
                celestia_endpoint: "http://celestia.env".into(),
                celestia_bearer_token: "envbearertoken".into(),
                gas_limit: 5555,
                disable_writing: true,
                sequencer_block_time_ms: 5555,
                validator_key_file: "/env/key".into(),
                rpc_port: 5555,
                p2p_port: 5555,
                bootnodes: Some(vec![
                    "/cli/bootnode3".to_string(),
                    "/cli/bootnode4".to_string(),
                ]),
                libp2p_private_key: Some("envlibp2p.key".to_string()),
                log: "env=debug".into(),
            };
            assert_eq!(expected, actual);
            Ok(())
        })
    }

    #[test]
    fn astria_log_overrides_rust_log() {
        Jail::expect_with(|jail| {
            jail.set_env("RUST_LOG", "rust=trace");
            jail.set_env("ASTRIA_SEQUENCER_RELAYER_LOG", "env=debug");
            let cli_args = make_args(NO_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                log: "env=debug".into(),
                ..Config::default()
            };
            assert_eq!(expected, actual);
            Ok(())
        })
    }
}
