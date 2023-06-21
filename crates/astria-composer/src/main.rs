use astria_composer::{
    config,
    searcher::Searcher,
    telemetry,
};
use color_eyre::eyre;
use tracing::error;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    telemetry::init(std::io::stdout).expect("failed to initialize tracing");

    let searcher = Searcher::new(cfg.searcher)?;
    let searcher_ask = tokio::spawn(searcher.run());

    tokio::select! {
        outcome = searcher_ask => {
            // TODO
            error!("searcher exited early: {:?}", outcome);
        }
    }

    Ok(())
}
