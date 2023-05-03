use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Result;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use log::{error, info};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::{mpsc, watch},
    time,
};

use astria_conductor::alert::Alert;
use astria_conductor::cli::Cli;
use astria_conductor::config::Config;
use astria_conductor::driver::{Driver, DriverCommand};
use astria_conductor::logger;

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}

async fn run() -> Result<()> {
    let args = Cli::parse();
    // logs
    logger::initialize(&args.log_level);

    // hierarchical config. cli args override Envars which override toml config values
    let conf: Config = Figment::new()
        .merge(Toml::file("ConductorConfig.toml"))
        .merge(Env::prefixed("ASTRIA_"))
        .merge(Serialized::defaults(args))
        .extract()?;

    log::info!("Using chain ID {}", conf.chain_id);
    log::info!("Using Celestia node at {}", conf.celestia_node_url);
    log::info!("Using execution node at {}", conf.execution_rpc_url);
    log::info!("Using Tendermint node at {}", conf.tendermint_url);

    let SignalReceiver {
        mut reload_rx,
        mut stop_rx,
    } = spawn_signal_handler();

    // spawn our driver
    let (alert_tx, mut alert_rx) = mpsc::unbounded_channel();
    let mut driver = Driver::new(conf, alert_tx).await?;
    let driver_tx = driver.cmd_tx.clone();

    tokio::task::spawn(async move {
        if let Err(e) = driver.run().await {
            panic!("Driver error: {}", e)
        }
    });

    // NOTE - this will most likely be replaced by an RPC server that will receive gossip
    //  messages from the sequencer
    let mut interval = time::interval(Duration::from_secs(3));

    loop {
        select! {
            // FIXME: The bias should only be on the signal channels. The the two
            //       handlers should have the same bias.
            biased;

            _ = stop_rx.changed() => {
                info!("shutting down conductor");
                if let Some(e) = driver_tx.send(DriverCommand::Shutdown).err() {
                    error!("error sending Shutdown command to driver: {}", e);
                }
                break;
            }

            _ = reload_rx.changed() => {
                info!("reloading is currently not implemented");
            }

            // handle alerts from the driver
            Some(alert) = alert_rx.recv() => {
                match alert {
                    Alert::DriverError(error_string) => {
                        error!("error: {}", error_string);
                        break;
                    }
                    Alert::BlockReceived{block_height} => {
                        info!("block received from DA layer; DA layer height: {}", block_height);
                    }
                }
            }
            // request new blocks every X seconds
            _ = interval.tick() => {
                if let Some(e) = driver_tx.send(DriverCommand::GetNewBlocks).err() {
                    // the only error that can happen here is SendError which occurs
                    // if the driver's receiver channel is dropped
                    error!("error sending GetNewBlocks command to driver: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}

struct SignalReceiver {
    reload_rx: watch::Receiver<()>,
    stop_rx: watch::Receiver<()>,
}

fn spawn_signal_handler() -> SignalReceiver {
    let (stop_tx, stop_rx) = watch::channel(());
    let (reload_tx, reload_rx) = watch::channel(());
    tokio::spawn(async move {
        let mut sighup = signal(SignalKind::hangup()).expect(
            "setting a SIGHUP listener should always work on linux; is this running on linux?",
        );
        let mut sigint = signal(SignalKind::interrupt()).expect(
            "setting a SIGINT listener should always work on linux; is this running on linux?",
        );
        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on linux; is this running on linux?",
        );
        loop {
            select! {
                _ = sighup.recv() => {
                    log::info!("received SIGHUP");
                    let _ = reload_tx.send(());
                }
                _ = sigint.recv() => {
                    log::info!("received SIGINT");
                    let _ = stop_tx.send(());
                }
                _ = sigterm.recv() => {
                    log::info!("received SIGTERM");
                    let _ = stop_tx.send(());
                }
            }
        }
    });

    SignalReceiver { reload_rx, stop_rx }
}
