use astria_composer::{
    config,
    searcher::Searcher,
    telemetry,
};
use color_eyre::eyre;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    telemetry::init(std::io::stdout).expect("failed to initialize tracing");

    let searcher = Searcher::new(cfg.searcher)?;
    let searcher_task = tokio::spawn(searcher.run());

    tokio::select! {
        _outcome = searcher_task => {
            // todo!("report searcher task exit")
        }
    }

    Ok(())
}
