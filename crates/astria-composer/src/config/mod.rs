use clap::Parser;
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
pub mod searcher;

// TODO: add more default values
// potentially move to a separate module so it can be imported into searcher and block_builder?
const DEFAULT_LOG: &str = "info";

#[derive(Debug, Deserialize, Serialize, PartialEq)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    pub log: String,
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
            log: DEFAULT_LOG.into(),
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
    use clap::Parser;
    use color_eyre::eyre;
    use figment::Jail;

    use super::{
        cli,
        Config,
    };

    const NO_CLI_ARGS: &str = "astria-composer";
    const ALL_CLI_ARGS: &str = r#"
astria-composer
    --log=debug
    --searcher-endpoint=foo
    "#;

    fn make_args(args: &str) -> eyre::Result<cli::Args, clap::Error> {
        cli::Args::try_parse_from(str::split_ascii_whitespace(args))
    }

    fn set_all_env(jail: &mut Jail) {
        jail.set_env("", "");
        todo!("set all env vars here")
    }

    #[test]
    fn cli_overrides_all() {
        Jail::expect_with(|jail| {
            set_all_env(jail);
            let cli_args = make_args(ALL_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                log: todo!(),
                searcher: todo!(),
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
                log: todo!(),
                searcher: todo!(),
            };
            assert_eq!(expected, actual);
            Ok(())
        })
    }

    #[test]
    fn astria_log_overrides_rust_log() {
        Jail::expect_with(|jail| {
            jail.set_env("RUST_LOG", "rust=trace");
            jail.set_env("ASTRIA_COMPOSER_LOG", "env=debug");
            let cli_args = make_args(NO_CLI_ARGS).unwrap();
            let actual = Config::with_cli(cli_args).unwrap();
            let expected = Config {
                log: "env=debug".into(),
                ..Config::default()
            };
            assert_eq!(expected, actual);
            Ok(())
        });
    }
}
