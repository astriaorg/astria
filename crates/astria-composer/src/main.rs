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
    telemetry::init(std::io::stdout, &cfg.log).expect("failed to initialize tracing");

    info!(?cfg, "starting astria-composer");
    // let composer = Composer::new(&cfg);

    let _searcher = Searcher::new(&cfg).await?.run().await;

    Ok(())
}
