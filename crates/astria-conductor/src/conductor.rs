use std::time::Duration;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::watch,
    task::JoinHandle,
    time,
};
use tracing::{
    error,
    info,
};

use crate::{
    driver::DriverCommand,
    Config,
    Driver,
};
pub struct Conductor {
    signals: SignalReceiver,
    driver: Driver,
    executor_join_handle: JoinHandle<eyre::Result<()>>,
    reader_join_handle: Option<JoinHandle<eyre::Result<()>>>,
}

impl Conductor {
    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        let signals = spawn_signal_handler();
        // spawn our driver
        let (driver, executor_join_handle, reader_join_handle) = Driver::new(cfg)
            .await
            .wrap_err("failed initializing driver")?;
        Ok(Self {
            signals,
            driver,
            executor_join_handle,
            reader_join_handle,
        })
    }

    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            signals:
                SignalReceiver {
                    mut reload_rx,
                    mut stop_rx,
                },
            driver,
            executor_join_handle: mut executor,
            reader_join_handle: mut reader,
        } = self;
        let driver_tx = driver.cmd_tx.clone();
        let mut driver = tokio::spawn(driver.run());
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

                res = &mut driver => {
                    match res {
                        Ok(Ok(())) => error!("driver task exited unexpectedly"),
                        Ok(Err(e)) => error!(error.msg = %e, error.cause = ?e, "driver exited with error"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "driver task failed"),
                    }
                    break;
                }

                res = &mut executor => {
                    match res {
                        Ok(Ok(())) => error!("executor task exited unexpectedly"),
                        Ok(Err(e)) => error!(error.msg = %e, error.cause = ?e, "executor exited with error"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "executor task failed"),
                    }
                    break;
                }


                res = async { reader.as_mut().unwrap().await }, if reader.is_some() => {
                    match res {
                        Ok(Ok(())) => error!("reader task exited unexpectedly"),
                        Ok(Err(e)) => error!(error.msg = %e, error.cause = ?e, "reader exited with error"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "reader task failed"),
                    }
                    break;
                }
            }
        }

        Ok(())
    }
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
            "setting a SIGHUP listener should always work on unix; is this running on unix?",
        );
        let mut sigint = signal(SignalKind::interrupt()).expect(
            "setting a SIGINT listener should always work on unix; is this running on unix?",
        );
        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
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
