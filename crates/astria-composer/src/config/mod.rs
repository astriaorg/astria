use clap::Parser;
use figment::{
    map,
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
pub mod searcher;

// TODO: add more default values
// potentially move to a separate module so it can be imported into searcher and block_builder?
const DEFAULT_LOG: &str = "info";

fn default_log() -> String {
    DEFAULT_LOG.into()
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    #[serde(default = "default_log")]
    pub log: String,

    /// Config for Searcher service
    #[serde(default = "searcher::Config::default")]
    pub searcher: searcher::Config,
    // TODO: add block_builder
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    fn with_cli(cli_config: cli::Args) -> Result<Config, figment::Error> {
        let rust_log = Env::prefixed("RUST_").split("_").only(&["log"]);

        // parse searcher args
        let searcher = searcher::Config::with_cli(cli_config.clone())?;

        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(rust_log)
            .merge(Env::prefixed("ASTRIA_COMPOSER_"))
            .merge(Serialized::defaults(cli_config))
            .merge(Serialized::defaults(map!["searcher" => searcher]))
            .extract()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: default_log(),
            searcher: searcher::Config::default(),
        }
    }
}

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
    let cmd = <cli::Args as clap::CommandFactory>::command();
    // We generate `cmd` after making sure the command parses successfully
    // to access the binary name and version. It is not possible to get this
    // information through the parsed type itself.
    eprintln!(
        "running {name}:{version}",
        name = cmd.get_name(),
        version = cmd.get_version().unwrap_or("<no-version-set>"),
    );
    Config::with_cli(cli_config)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use clap::Parser;
    use color_eyre::eyre;
    use figment::Jail;

    use super::{
        cli,
        Config,
    };
    use crate::{
        config::searcher,
        types::rollup::ChainId,
    };

    const NO_CLI_ARGS: &str = "astria-composer";
    const ALL_CLI_ARGS: &str = r#"
astria-composer
    --log debug
    --sequencer-url 127.0.0.1:1310
    --searcher-api-url 127.0.0.1:7070
    --searcher-chain-id clinet
    --searcher-execution-rpc-url 127.0.0.1:60061
    "#;

    fn make_args(args: &str) -> eyre::Result<cli::Args, clap::Error> {
        cli::Args::try_parse_from(str::split_ascii_whitespace(args))
    }

    fn set_all_env(jail: &mut Jail) {
        jail.set_env("ASTRIA_COMPOSER_LOG", "warn");
        jail.set_env("ASTRIA_COMPOSER_SEQUENCER_URL", "127.0.0.1:1210");
        jail.set_env("ASTRIA_COMPOSER_SEARCHER_API_URL", "127.0.0.1:5050");
        jail.set_env("ASTRIA_COMPOSER_SEARCHER_CHAIN_ID", "envnet");
        jail.set_env(
            "ASTRIA_COMPOSER_SEARCHER_EXECUTION_RPC_URL",
            "127.0.0.1:40041",
        );
    }

    #[test]
    fn cli_overrides_all() {
        Jail::expect_with(|jail| {
            set_all_env(jail);
            let cli_args = make_args(ALL_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                log: "debug".into(),
                searcher: searcher::Config {
                    sequencer_url: "127.0.0.1:1310".parse().unwrap(),
                    api_url: "127.0.0.1:7070".parse().unwrap(),
                    chain_id: ChainId::from_str("clinet").unwrap(),
                    execution_rpc_url: "127.0.0.1:60061".parse().unwrap(),
                },
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
                log: "warn".into(),
                searcher: searcher::Config {
                    sequencer_url: "127.0.0.1:1210".parse().unwrap(),
                    api_url: "127.0.0.1:5050".parse().unwrap(),
                    chain_id: ChainId::from_str("envnet").unwrap(),
                    execution_rpc_url: "127.0.0.1:40041".parse().unwrap(),
                },
            };
            assert_eq!(expected, actual);
            Ok(())
        })
    }

    #[test]
    fn astria_log_overrides_rust_log() {
        Jail::expect_with(|jail| {
            jail.set_env("RUST_LOG", "rust=trace");
            jail.set_env("ASTRIA_COMPOSER_LOG", "debug");
            let cli_args = make_args(NO_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                log: "debug".into(),
                ..Config::default()
            };
            assert_eq!(expected, actual);
            Ok(())
        });
    }
}
