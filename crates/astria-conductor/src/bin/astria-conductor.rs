use std::time::Duration;

use astria_conductor::{
    alert::Alert,
    cli::Cli,
    config::Config,
    driver::{
        Driver,
        DriverCommand,
    },
    logger,
};
use clap::Parser;
use color_eyre::eyre::{
    eyre,
    Result,
};
use figment::{
    providers::{
        Env,
        Format,
        Serialized,
        Toml,
    },
    Figment,
};
use log::{
    error,
    info,
};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::{
        mpsc,
        watch,
    },
    time,
};

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
    let (mut driver, executor_join_handle, reader_join_handle) =
        Driver::new(conf, alert_tx.clone()).await?;
    let driver_tx = driver.cmd_tx.clone();

    tokio::task::spawn(async move {
        if let Err(e) = driver.run().await {
            panic!("Driver error: {}", e)
        }
    });

    let executor_alert_tx = alert_tx.clone();
    tokio::task::spawn(async move {
        match executor_join_handle.await {
            Ok(run_res) => match run_res {
                Ok(_) => {
                    _ = executor_alert_tx.send(Alert::DriverError(eyre!(
                        "executor task exited unexpectedly"
                    )));
                }
                Err(e) => {
                    _ = executor_alert_tx.send(Alert::DriverError(eyre!(
                        "executor exited with error: {}",
                        e
                    )));
                }
            },
            Err(e) => {
                _ = executor_alert_tx.send(Alert::DriverError(eyre!(
                    "received JoinError from executor task: {}",
                    e
                )));
            }
        }
    });

    if reader_join_handle.is_some() {
        tokio::task::spawn(async move {
            match reader_join_handle.unwrap().await {
                Ok(run_res) => match run_res {
                    Ok(_) => {
                        _ = alert_tx
                            .send(Alert::DriverError(eyre!("reader task exited unexpectedly")));
                    }
                    Err(e) => {
                        _ = alert_tx
                            .send(Alert::DriverError(eyre!("reader exited with error: {}", e)));
                    }
                },
                Err(e) => {
                    _ = alert_tx.send(Alert::DriverError(eyre!(
                        "received JoinError from reader task: {}",
                        e
                    )));
                }
            }
        });
    }

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
                    Alert::BlockReceivedFromGossipNetwork{block_height} => {
                        info!("sequencer block received from p2p network; height: {}", block_height);
                    }
                    Alert::BlockReceivedFromDataAvailability{block_height} => {
                        info!("sequencer block received from DA layer; height: {}", block_height);
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

    SignalReceiver {
        reload_rx,
        stop_rx,
    }
}
