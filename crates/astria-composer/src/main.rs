use astria_composer::{
    config,
    searcher::Searcher,
    telemetry,
};
use color_eyre::eyre;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    telemetry::init(&cfg.log, std::io::stdout).expect("failed to initialize tracing");

    info!(?cfg, "starting astria-composer");

    let _searcher = Searcher::new(&cfg.searcher).await?.run().await;

    Ok(())
}
