use clap::Parser;
use color_eyre::eyre;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = astria_cli::Cli::parse();
    // println!("{:?}", args.log_level);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(args.log_level.to_string()));

    tracing_subscriber::fmt()
        .pretty()
        // .json() // TODO: Enable JSON outputs
        .with_env_filter(env_filter)
        // .with_current_span(false)
        // .with_span_list(false)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(std::io::stderr)
        .init();

    astria_cli::Cli::run().await
}
