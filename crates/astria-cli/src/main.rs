use std::process::ExitCode;

use astria_cli::{
    cli::Cli,
    commands,
};
use clap::Command;
use color_eyre::eyre;

mod query;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .pretty()
        .with_writer(std::io::stderr)
        .init();

    let matches = Command::new("astria-cli")
        .subcommand(query::command())
        // .subcommand(command2::command(&config))
        .get_matches();

    match matches.subcommand() {
        Some(("query", args)) => query::run(args).await.expect("Could not run query command"),
        // Some(("command2", sub_matches)) => command2::run(sub_matches, &config),
        _ => {
            return ExitCode::FAILURE;
        }
    }

    // if let Err(err) = run().await {
    //     eprintln!("{err:?}");
    //     return ExitCode::FAILURE;
    // }

    ExitCode::SUCCESS
}

async fn run() -> eyre::Result<()> {
    let args = Cli::get_args()?;
    commands::run(args).await
}
