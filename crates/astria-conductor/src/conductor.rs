use astria_sequencer_types::ChainId;
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
    sync::{
        mpsc,
        oneshot,
        watch,
    },
};
use tracing::{
    error,
    info,
};

use crate::{
    executor::Executor,
    Config,
    Driver,
};
pub struct Conductor {
    signals: SignalReceiver,
    executor: Executor,
    executor_shutdown: oneshot::Sender<()>,
    driver: Driver,
    driver_shutdown: oneshot::Sender<()>,
}

impl Conductor {
    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        let signals = spawn_signal_handler();
        // spawn our driver
        let (executor_tx, executor_rx) = mpsc::unbounded_channel();
        let (executor_shutdown_tx, executor_shutdown_rx) = oneshot::channel();
        let executor = Executor::new(
            &cfg.execution_rpc_url,
            ChainId::new(cfg.chain_id.as_bytes().to_vec()).wrap_err("failed to create chain ID")?,
            cfg.disable_empty_block_execution,
            executor_rx,
            executor_shutdown_rx,
        )
        .await
        .wrap_err("failed to construct executor")?;

        let (driver_shutdown_tx, driver_shutdown_rx) = oneshot::channel();
        let driver = Driver::new(cfg, driver_shutdown_rx, executor_tx)
            .await
            .wrap_err("failed initializing driver")?;
        Ok(Self {
            signals,
            driver,
            driver_shutdown: driver_shutdown_tx,
            executor,
            executor_shutdown: executor_shutdown_tx,
        })
    }

    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            signals:
                SignalReceiver {
                    mut reload_rx,
                    mut stop_rx,
                },
            executor,
            executor_shutdown,
            driver,
            driver_shutdown,
        } = self;

        let mut driver = tokio::spawn(driver.run_until_stopped());
        let mut executor = tokio::spawn(executor.run_until_stopped());

        loop {
            select! {
                // FIXME: The bias should only be on the signal channels. The the two
                //       handlers should have the same bias.
                biased;

                _ = stop_rx.changed() => {
                    info!("shutting down conductor");
                    break;
                }

                _ = reload_rx.changed() => {
                    info!("reloading is currently not implemented");
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
                        Ok(()) => error!("executor task exited unexpectedly"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "executor task failed"),
                    }
                    break;
                }
            }
        }
        let _ = executor_shutdown.send(());
        let _ = driver_shutdown.send(());

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
