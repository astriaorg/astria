use std::time::Duration;

use astria_conductor::{
    config,
    driver::{
        Driver,
        DriverCommand,
    },
    telemetry,
};
use color_eyre::eyre::{Result, Context};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::watch,
    time,
};
use tracing::{
    error,
    info,
    instrument,
};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
            eprintln!("Exited with error: {e}");
            std::process::exit(2);
    };
    Ok(())
}

#[instrument(name = "astria_conductor::run")]
async fn run() -> Result<()> {
    let conf = config::get().wrap_err("failed to read config")?;

    telemetry::init(std::io::stdout, &conf.log).wrap_err("failed to initialize telemetry")?;

    info!("Using chain ID {}", conf.chain_id);
    info!("Using Celestia node at {}", conf.celestia_node_url);
    info!("Using execution node at {}", conf.execution_rpc_url);
    info!("Using Tendermint node at {}", conf.tendermint_url);

    let SignalReceiver {
        mut reload_rx,
        mut stop_rx,
    } = spawn_signal_handler();

    // spawn our driver
    let (mut driver, executor_join_handle, reader_join_handle) = Driver::new(conf).await?;
    let driver_tx = driver.cmd_tx.clone();

    tokio::task::spawn(async move {
        if let Err(e) = driver.run().await {
            panic!("Driver error: {}", e)
        }
    });

    tokio::task::spawn(async move {
        match executor_join_handle.await {
            Ok(run_res) => match run_res {
                Ok(_) => {
                    error!("executor task exited unexpectedly");
                }
                Err(e) => {
                    error!("executor exited with error: {}", e);
                }
            },
            Err(e) => {
                error!("received JoinError from executor task: {}", e);
            }
        }
    });

    if reader_join_handle.is_some() {
        tokio::task::spawn(async move {
            match reader_join_handle.unwrap().await {
                Ok(run_res) => match run_res {
                    Ok(_) => {
                        error!("reader task exited unexpectedly");
                    }
                    Err(e) => {
                        error!("reader exited with error: {}", e);
                    }
                },
                Err(e) => {
                    error!("received JoinError from reader task: {}", e);
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
                    error!(
                        error.msg = %e,
                        error.cause = ?e,
                        "error sending Shutdown command to driver"
                    );
                }
                break;
            }

            _ = reload_rx.changed() => {
                info!("reloading is currently not implemented");
            }
            // request new blocks every X seconds
            _ = interval.tick() => {
                if let Some(e) = driver_tx.send(DriverCommand::GetNewBlocks).err() {
                    // the only error that can happen here is SendError which occurs
                    // if the driver's receiver channel is dropped
                    error!(
                        error.msg = %e,
                        error.cause = ?e,
                        "error sending GetNewBlocks command to driver"
                    );
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
                    info!("received SIGHUP");
                    let _ = reload_tx.send(());
                }
                _ = sigint.recv() => {
                    info!("received SIGINT");
                    let _ = stop_tx.send(());
                }
                _ = sigterm.recv() => {
                    info!("received SIGTERM");
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
