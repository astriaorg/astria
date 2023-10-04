use std::{
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use astria_sequencer_types::{
    ChainId,
    Namespace,
};
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
    task::{
        spawn_local,
        LocalSet,
    },
    time::timeout,
};
use tokio_util::task::JoinMap;
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    block_verifier::BlockVerifier,
    client_provider::ClientProvider,
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
    sequencer_client_pool: deadpool::managed::Pool<ClientProvider>,
}

impl Conductor {
    const DATA_AVAILABILITY: &str = "data_availability";
    const EXECUTOR: &str = "executor";
    const SEQUENCER: &str = "sequencer";

    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        let mut tasks = JoinMap::new();
        let mut shutdown_channels = HashMap::new();

        let signals = spawn_signal_handler();

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

        let client_provider = ClientProvider::new(&cfg.sequencer_url)
            .await
            .wrap_err("failed initializing sequencer client provider")?;
        let sequencer_client_pool = deadpool::managed::Pool::builder(client_provider)
            .max_size(50)
            .build()
            .wrap_err("failed to create sequencer client pool")?;

        let (sequencer_shutdown_tx, sequencer_shutdown_rx) = oneshot::channel();
        let sequencer_reader = sequencer::Reader::new(
            cfg.initial_sequencer_block_height,
            sequencer_client_pool.clone(),
            sequencer_shutdown_rx,
            executor_tx.clone(),
        );

        tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
        shutdown_channels.insert(Self::SEQUENCER, sequencer_shutdown_tx);

        if !cfg.disable_finalization {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let block_verifier = BlockVerifier::new(sequencer_client_pool.clone());
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
            sequencer_client_pool,
            shutdown_channels,
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
            sequencer_client_pool,
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
            }
        }

        info!("sending shutdown command to all tasks");
        for (_, channel) in shutdown_channels {
            let _ = channel.send(());
        }

        sequencer_client_pool.close();

        // wait 5 seconds for all tasks to shut down
        // put the tasks into an Rc to make them 'static
        let mut tasks = Rc::new(tasks);
        let local_set = LocalSet::new();
        local_set
            .run_until(async {
                let mut tasks = tasks.clone();
                let _ = timeout(
                    Duration::from_secs(5),
                    spawn_local(async move {
                        while Rc::get_mut(&mut tasks)
                            .expect(
                                "only one Rc to the conductor tasks should exist; this is a bug",
                            )
                            .join_next()
                            .await
                            .is_some()
                        {}
                    }),
                )
                .await;
            })
            .await;

        if !tasks.is_empty() {
            warn!(
                number = tasks.len(),
                "aborting tasks that haven't shutdown yet"
            );
            Rc::get_mut(&mut tasks)
                .expect("only one Rc to the conductor tasks should exist; this is a bug")
                .shutdown()
                .await;
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
