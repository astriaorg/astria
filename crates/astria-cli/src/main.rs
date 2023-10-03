use std::process::ExitCode;

use astria_cli::{
    cli::Cli,
    commands,
};
use color_eyre::eyre;

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("{}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run() -> eyre::Result<()> {
    let args = Cli::get_args()?;
    commands::run(args)
}
