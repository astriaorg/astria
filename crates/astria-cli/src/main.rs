use std::process::ExitCode;

use astria_cli::{
    cli::Cli,
    commands,
};
use color_eyre::eyre;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .pretty()
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = run().await {
        eprintln!("{err:?}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run() -> eyre::Result<()> {
    let args = Cli::get_args()?;
    commands::run(args).await
}
