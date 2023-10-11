use std::process::ExitCode;

use astria_cli::{
    cli::Cli,
    commands,
};
use color_eyre::{
    eyre,
    eyre::Context,
};

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("{err:#?}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Run our asynchronous command code in a blocking manner
fn run() -> eyre::Result<()> {
    let rt = tokio::runtime::Runtime::new().wrap_err("failed to create a new runtime")?;

    rt.block_on(async {
        let args = Cli::get_args()?;
        commands::run(args).await
    })
}
