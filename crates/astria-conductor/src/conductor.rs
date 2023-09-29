use astria_sequencer_types::{
    ChainId,
    Namespace,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use sequencer_client::WebSocketClient;
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
    task::JoinHandle,
};
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    block_verifier::BlockVerifier,
    data_availability,
    executor::Executor,
    sequencer,
    Config,
};
pub struct Conductor {
    signals: SignalReceiver,
    executor: Executor,
    executor_shutdown: oneshot::Sender<()>,
    sequencer_reader: sequencer::Reader,
    sequencer_reader_shutdown: oneshot::Sender<()>,
    data_availability_reader: Option<data_availability::Reader>,
    data_availability_reader_shutdown: Option<oneshot::Sender<()>>,
    sequencer_websocket_driver: JoinHandle<Result<(), sequencer_client::tendermint::Error>>,
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

        let (sequencer_client, sequencer_websocket_driver) = {
            let (client, driver) = WebSocketClient::new(&*cfg.sequencer_url).await.wrap_err(
                "failed constructing a cometbft websocket client to read off sequencer",
            )?;
            let driver_handle = tokio::spawn(async move { driver.run().await });
            (client, driver_handle)
        };

        let (sequencer_shutdown_tx, sequencer_shutdown_rx) = oneshot::channel();
        let sequencer_reader = sequencer::Reader::new(
            sequencer_client.clone(),
            sequencer_shutdown_rx,
            executor_tx.clone(),
        )
        .await
        .wrap_err("failed initializing driver")?;

        let mut data_availability_reader = None;
        let mut data_availability_reader_shutdown = None;
        if !cfg.disable_finalization {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let block_verifier = BlockVerifier::new(sequencer_client.clone());
            data_availability_reader = Some(
                data_availability::Reader::new(
                    &cfg.celestia_node_url,
                    &cfg.celestia_bearer_token,
                    std::time::Duration::from_secs(3),
                    executor_tx.clone(),
                    block_verifier,
                    Namespace::from_slice(cfg.chain_id.as_bytes()),
                    shutdown_rx,
                )
                .await
                .wrap_err("failed constructing data availability reader")?,
            );
            data_availability_reader_shutdown = Some(shutdown_tx);
        };

        Ok(Self {
            signals,
            sequencer_reader,
            sequencer_reader_shutdown: sequencer_shutdown_tx,
            executor,
            executor_shutdown: executor_shutdown_tx,
            data_availability_reader,
            data_availability_reader_shutdown,
            sequencer_websocket_driver,
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
            data_availability_reader,
            data_availability_reader_shutdown,
            sequencer_reader,
            sequencer_reader_shutdown,
            mut sequencer_websocket_driver,
        } = self;

        let mut sequencer_reader = tokio::spawn(sequencer_reader.run_until_stopped());
        let mut executor = tokio::spawn(executor.run_until_stopped());
        let mut data_availability_reader = if let Some(reader) = data_availability_reader {
            Some(tokio::spawn(reader.run()))
        } else {
            None
        };

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

                res = &mut sequencer_reader => {
                    match res {
                        Ok(Ok(())) => error!("sequencer reader exited unexpectedly; shutting down"),
                        Ok(Err(e)) => error!(error.msg = %e, error.cause = ?e, "sequencer reader exited with error; shutting down"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "sequencer reader task failed; shutting down"),
                    }
                    break;
                }

                res = async { data_availability_reader.as_mut().unwrap().await }, if data_availability_reader.is_some() => {
                    match res {
                        Ok(Ok(())) => warn!("data availability reader exited unexpectedly; shutting down"),
                        Ok(Err(e)) => warn!(err.message = %e, err.cause = ?e, "data availability reader exited with error; shutting down"),
                        Err(e) => warn!(err.cause = ?e, "data availability task failed; shutting down"),
                    }
                    break;
                }

                res = &mut executor => {
                    match res {
                        Ok(()) => error!("executor task exited unexpectedly; shutting down"),
                        Err(e) => error!(error.msg = %e, error.cause = ?e, "executor task failed; shutting down"),
                    }
                    break;
                }

                driver_res = &mut sequencer_websocket_driver => {
                    match driver_res {
                        Ok(Ok(())) => warn!("sequencer client websocket driver exited unexpectedly"),
                        Ok(Err(e)) => warn!(err.message = %e, err.cause = ?e, "sequencer client websocket driver exited with error"),
                        Err(e) => warn!(err.cause = ?e, "sequencer client driver task failed"),
                    }
                    break;
                }
            }
        }
        let _ = executor_shutdown.send(());
        let _ = sequencer_reader_shutdown.send(());
        let _ = data_availability_reader_shutdown.map(|shutdown| shutdown.send(()));

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
