use astria_sequencer_relayer::{
    config,
    telemetry,
    SequencerRelayer,
};
use eyre::WrapErr as _;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    let (cfg_ser, cfg_dbg) = match serde_json::to_string(&cfg) {
        Ok(cfg_ser) => {
            eprintln!("config:\n{cfg_ser}");
            (Some(cfg_ser), None)
        }
        Err(e) => {
            eprintln!(
                "failed serializing config to json; will use debug formatting; error:\n{e:?}"
            );
            let cfg_dbg = format!("{cfg:?}");
            eprintln!("config:\n{cfg_dbg}");
            (None, Some(cfg_dbg))
        }
    };
    telemetry::init(std::io::stdout, &cfg.log).expect("failed to setup telemetry");
    info!(
        config = cfg_ser,
        config.debug = cfg_dbg,
        "initializing sequencer relayer"
    );

    SequencerRelayer::new(cfg)
        .wrap_err("failed to initialize sequencer relayer")?
        .run()
        .await;

    Ok(())
}
