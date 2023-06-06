//! CLI definition and entrypoint to executable
use std::str::FromStr;

use clap::{
    ArgAction,
    Args,
    Parser,
    Subcommand,
};
use reth_tracing::{
    tracing::{
        metadata::LevelFilter,
        Level,
        Subscriber,
    },
    tracing_subscriber::{
        filter::Directive,
        registry::LookupSpan,
    },
    BoxedLayer,
    FileWorkerGuard,
};

use crate::{
    // chain,
    // db,
    dirs::{
        LogsDir,
        PlatformPath,
    },
    node,
    runner::CliRunner,
};

/// Parse CLI options, set up logging and run the chosen command.
pub fn run() -> eyre::Result<()> {
    let opt = Cli::parse();

    // let mut layers = vec![reth_tracing::stdout(opt.verbosity.directive())];
    // if let Some((layer, _guard)) = opt.logs.layer() {
    //     layers.push(layer);
    // }
    // reth_tracing::init(layers);

    let runner = CliRunner::default();

    match opt.command {
        Commands::Node(command) => runner.run_command_until_exit(|ctx| command.execute(ctx)),
        // Commands::Init(command) => runner.run_blocking_until_ctrl_c(command.execute()),
    }
}

/// Commands to be executed
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the node
    #[command(name = "node")]
    Node(node::Command),
    // /// Initialize the database from a genesis file.
    // #[command(name = "init")]
    // Init(chain::InitCommand),
}

#[derive(Debug, Parser)]
#[command(author, version = "0.1", about = "Reth", long_about = None)]
struct Cli {
    /// The command to run
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    logs: Logs,
    // #[clap(flatten)]
    // verbosity: Verbosity,
}

/// The log configuration.
#[derive(Debug, Args)]
#[command(next_help_heading = "Logging")]
pub struct Logs {
    /// The flag to enable persistent logs.
    #[arg(long = "log.persistent", global = true, conflicts_with = "journald")]
    persistent: bool,

    /// The path to put log files in.
    #[arg(
        long = "log.directory",
        value_name = "PATH",
        global = true,
        default_value_t,
        conflicts_with = "journald"
    )]
    log_directory: PlatformPath<LogsDir>,

    /// Log events to journald.
    #[arg(long = "log.journald", global = true, conflicts_with = "log_directory")]
    journald: bool,

    /// The filter to use for logs written to the log file.
    #[arg(
        long = "log.filter",
        value_name = "FILTER",
        global = true,
        default_value = "debug"
    )]
    filter: String,
}

impl Logs {
    /// Builds a tracing layer from the current log options.
    pub fn layer<S>(&self) -> Option<(BoxedLayer<S>, Option<FileWorkerGuard>)>
    where
        S: Subscriber,
        for<'a> S: LookupSpan<'a>,
    {
        let directive = Directive::from_str(self.filter.as_str())
            .unwrap_or_else(|_| Directive::from_str("debug").unwrap());

        if self.journald {
            Some((
                reth_tracing::journald(directive).expect("Could not connect to journald"),
                None,
            ))
        } else if self.persistent {
            let (layer, guard) = reth_tracing::file(directive, &self.log_directory, "reth.log");
            Some((layer, Some(guard)))
        } else {
            None
        }
    }
}

/// The verbosity settings for the cli.
#[derive(Debug, Copy, Clone, Args)]
#[command(next_help_heading = "Display")]
pub struct Verbosity {
    /// Set the minimum log level.
    ///
    /// -v      Errors
    /// -vv     Warnings
    /// -vvv    Info
    /// -vvvv   Debug
    /// -vvvvv  Traces (warning: very verbose!)
    #[clap(short, long, action = ArgAction::Count, global = true, default_value_t = 3, verbatim_doc_comment, help_heading = "Display")]
    verbosity: u8,

    /// Silence all log output.
    #[clap(
        long,
        alias = "silent",
        short = 'q',
        global = true,
        help_heading = "Display"
    )]
    quiet: bool,
}

impl Verbosity {
    /// Get the corresponding [Directive] for the given verbosity, or none if the verbosity
    /// corresponds to silent.
    pub fn directive(&self) -> Directive {
        if self.quiet {
            LevelFilter::OFF.into()
        } else {
            let level = match self.verbosity - 1 {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                _ => Level::TRACE,
            };

            format!("reth::cli={level}").parse().unwrap()
        }
    }
}

