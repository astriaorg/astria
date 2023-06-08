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
    Serialize,
};
mod cli;

const DEFAULT_BLOCK_TIME: u64 = 3000;
const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26659";
const DEFAULT_SEQUENCER_ENDPOINT: &str = "http://localhost:1317";
const DEFAULT_VALIDATOR_KEY_FILE: &str = ".metro/config/priv_validator_key.json";

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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub gas_limit: u64,
    pub disable_writing: bool,
    pub block_time: u64,
    pub validator_key_file: String,
    pub rpc_port: u16,
    pub p2p_port: u16,
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
            sequencer_endpoint: DEFAULT_SEQUENCER_ENDPOINT.into(),
            gas_limit: crate::da::DEFAULT_PFD_GAS_LIMIT,
            disable_writing: false,
            block_time: DEFAULT_BLOCK_TIME,
            validator_key_file: DEFAULT_VALIDATOR_KEY_FILE.into(),
            rpc_port: DEFAULT_RPC_LISTEN_PORT,
            p2p_port: DEFAULT_GOSSIP_PORT,
            log: DEFAULT_LOG_DIRECTIVE.into(),
        }
    }
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
    --sequencer-endpoint http://sequencer.cli
    --celestia-endpoint http://celestia.cli
    --gas-limit 9999
    --disable-writing
    --block-time 9999
    --validator-key-file /cli/key
    --rpc-port 9999
    --p2p-port 9999
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
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_GAS_LIMIT", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_DISABLE_WRITING", true);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_BLOCK_TIME", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_VALIDATOR_KEY_FILE", "/env/key");
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_RPC_PORT", 5555);
        jail.set_env("ASTRIA_SEQUENCER_RELAYER_P2P_PORT", 5555);
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
                gas_limit: 9999,
                disable_writing: true,
                block_time: 9999,
                validator_key_file: "/cli/key".into(),
                rpc_port: 9999,
                p2p_port: 9999,
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
                gas_limit: 5555,
                disable_writing: true,
                block_time: 5555,
                validator_key_file: "/env/key".into(),
                rpc_port: 5555,
                p2p_port: 5555,
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
