use astria_sequencer_relayer::{
    config,
    SequencerRelayer,
};
use eyre::WrapErr as _;
use tracing::{
    info,
    warn,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    tracing_subscriber::fmt().with_env_filter(&cfg.log).init();
    let cfg_json = serde_json::to_string(&cfg).unwrap_or_else(|e| {
        warn!(
            error = ?e,
            "failed serializing config as json; will use debug formatting"
        );
        format!("{cfg:?}")
    });
    info!(config = cfg_json, "starting astria-sequencer-relayer");

    SequencerRelayer::new(cfg)
        .wrap_err("failed to initialize sequencer relayer")?
        .run()
        .await;

    Ok(())
}
