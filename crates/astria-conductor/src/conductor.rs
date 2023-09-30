use std::collections::HashMap;

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
use tokio_util::task::JoinMap;
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
    /// the different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,
    /// channels to the long-running tasks to shut them down gracefully
    shutdown_channels: HashMap<&'static str, oneshot::Sender<()>>,
    sequencer_websocket_driver: JoinHandle<Result<(), sequencer_client::tendermint::Error>>,
}

impl Conductor {
    const DATA_AVAILABILITY: &str = "data_availability";
    const EXECUTOR: &str = "executor";
    const SEQUENCER: &str = "sequencer";

    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        let mut tasks = JoinMap::new();
        let mut shutdown_channels = HashMap::new();

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

        tasks.spawn(Self::EXECUTOR, executor.run_until_stopped());
        shutdown_channels.insert(Self::EXECUTOR, executor_shutdown_tx);

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

        tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
        shutdown_channels.insert(Self::SEQUENCER, sequencer_shutdown_tx);

        if !cfg.disable_finalization {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let block_verifier = BlockVerifier::new(sequencer_client.clone());
            let data_availability_reader = data_availability::Reader::new(
                &cfg.celestia_node_url,
                &cfg.celestia_bearer_token,
                std::time::Duration::from_secs(3),
                executor_tx.clone(),
                block_verifier,
                Namespace::from_slice(cfg.chain_id.as_bytes()),
                shutdown_rx,
            )
            .await
            .wrap_err("failed constructing data availability reader")?;
            tasks.spawn(
                Self::DATA_AVAILABILITY,
                data_availability_reader.run_until_stopped(),
            );
            shutdown_channels.insert(Self::DATA_AVAILABILITY, shutdown_tx);
        };

        Ok(Self {
            signals,
            tasks,
            shutdown_channels,
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
            mut tasks,
            shutdown_channels,
            mut sequencer_websocket_driver,
        } = self;

        loop {
            select! {
                // FIXME: The bias should only be on the signal channels. The two handlers should have the same bias.
                biased;

                _ = stop_rx.changed() => {
                    info!("shutting down conductor");
                    break;
                }

                _ = reload_rx.changed() => {
                    info!("reloading is currently not implemented");
                }

                Some((name, res)) = tasks.join_next() => {
                    match res {
                        Ok(Ok(())) => error!(tak.name = name, "task exited unexpectedly, shutting down"),
                        Ok(Err(e)) => error!(task.name = name, error.msg = %e, error.cause = ?e, "task exited with error; shutting down"),
                        Err(e) => error!(task.name = name, error.msg = %e, error.cause = ?e, "task failed; shutting down"),
                    }
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

        for (_, channel) in shutdown_channels {
            let _ = channel.send(());
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
