use std::{
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use astria_sequencer_types::ChainId;
use base64::{
    display::Base64Display,
    engine::general_purpose::STANDARD,
};
use celestia_client::celestia_types::nmt::Namespace;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::future::Fuse;
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
    client_provider::{
        self,
        ClientProvider,
    },
    config::CommitLevel,
    data_availability::{
        self,
        CelestiaReaderConfig,
    },
    executor::Executor,
    sequencer,
    Config,
};

pub struct Conductor {
    /// The sequencer reader that is spawned after either after DA sync is completed,
    /// or when running in "SoftOnly".
    sequencer_reader: Option<sequencer::Reader>,

    /// The data availability reader that is spawned for DA sync, or after
    /// sequencer sync has been completed.
    data_availability_reader: Option<data_availability::Reader>,

    /// The object pool of sequencer clients that restarts the websocket connection
    /// on failure.
    sequencer_client_pool: deadpool::managed::Pool<ClientProvider>,

    /// Channels to the long-running tasks to shut them down gracefully
    shutdown_channels: HashMap<&'static str, oneshot::Sender<()>>,

    /// Listens for several unix signals and notifies its subscribers.
    signals: SignalReceiver,

    /// The channel over which the sequencer reader task notifies conductor that sync is completed.
    seq_sync_done: Fuse<oneshot::Receiver<()>>,

    /// The channel over which the sequencer reader task notifies conductor that sync is completed.
    da_sync_done: Fuse<oneshot::Receiver<()>>,

    /// The different long-running tasks that make up the conductor;
    tasks: JoinMap<&'static str, eyre::Result<()>>,

    /// The Execution Commit Level setting for the conductor
    execution_commit_level: CommitLevel,
}

impl Conductor {
    const DATA_AVAILABILITY: &'static str = "data_availability";
    const EXECUTOR: &'static str = "executor";
    const SEQUENCER: &'static str = "sequencer";

    /// Create a new [`Conductor`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    // TODO: refactor this function to be more readable and reduce the number of lines
    #[allow(clippy::too_many_lines)]
    pub async fn new(cfg: Config) -> eyre::Result<Self> {
        use futures::FutureExt;

        let mut tasks = JoinMap::new();
        let mut shutdown_channels = HashMap::new();

        let signals = spawn_signal_handler();

        // Spawn the executor task.
        let (executor_tx, soft_commit_height, firm_commit_height) = {
            let (block_tx, block_rx) = mpsc::unbounded_channel();
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let executor = Executor::new(
                &cfg.execution_rpc_url,
                ChainId::new(cfg.chain_id.as_bytes().to_vec())
                    .wrap_err("failed to create chain ID")?,
                cfg.initial_sequencer_block_height, // the sequencer block the rollup was start on
                block_rx,
                shutdown_rx,
            )
            .await
            .wrap_err("failed to construct executor")?;

            let executable_sequencer_block_height =
                executor.get_executable_sequencer_block_height();
            let executable_da_block_height = executor.get_executable_da_block_height();

            tasks.spawn(Self::EXECUTOR, executor.run_until_stopped());
            shutdown_channels.insert(Self::EXECUTOR, shutdown_tx);

            (
                block_tx,
                executable_sequencer_block_height,
                executable_da_block_height,
            )
        };

        let sequencer_client_pool = client_provider::start_pool(&cfg.sequencer_url)
            .wrap_err("failed to create sequencer client pool")?;

        // Spawn the sequencer task
        // Only spawn the sequencer::Reader if CommitLevel is not FirmOnly, also
        // send () to sync_done to start normal block execution behavior
        let mut seq_sync_done = futures::future::Fuse::terminated();
        let mut da_sync_done = futures::future::Fuse::terminated();

        let mut sequencer_reader = None;

        info!(execution_commit_level = ?cfg.execution_commit_level);

        match cfg.execution_commit_level {
            CommitLevel::SoftOnly => {
                info!("only syncing from sequencer");
                // kill the DA sync to only execute from sequencer
                let (sync_done_tx, sync_done_rx) = oneshot::channel();
                da_sync_done = sync_done_rx.fuse();
                let _ = sync_done_tx.send(());
            }
            CommitLevel::SoftAndFirm => {
                info!("syncing from DA then sequencer");
                // when running in soft and firm mode, a sync cycle from both DA
                // and sequencer are used. First we sync from DA up to the most
                // recent DA block, then we sync from most recent firm to the
                // latest soft commit from the sequencer. Neither sync is killed
                // in this mode.
            }
            CommitLevel::FirmOnly => {
                info!("only syncing from DA");
                // kill the sequencer sync to only execute from DA
                let (sync_done_tx, sync_done_rx) = oneshot::channel();
                seq_sync_done = sync_done_rx.fuse();
                let _ = sync_done_tx.send(());
            }
        }

        if !cfg.execution_commit_level.is_firm_only() {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let (sync_done_tx, sync_done_rx) = oneshot::channel();

            // The `sync_start_block_height` represents the height of the next
            // sequencer block that can be executed on top of the rollup state.
            // This value is derived by the Executor.
            let seq_reader = sequencer::Reader::new(
                soft_commit_height,
                sequencer_client_pool.clone(),
                shutdown_rx,
                executor_tx.clone(),
                sync_done_tx,
            );
            sequencer_reader = Some(seq_reader);
            shutdown_channels.insert(Self::SEQUENCER, shutdown_tx);
            seq_sync_done = sync_done_rx.fuse();
        }

        // Construct the data availability reader without spawning it.
        // It will be executed after sync is done.
        let mut data_availability_reader = None;

        // Only spawn the data_availability::Reader if CommitLevel is not SoftOnly
        if !cfg.execution_commit_level.is_soft_only() {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let (sync_done_tx, sync_done_rx) = oneshot::channel();

            let block_verifier = BlockVerifier::new(sequencer_client_pool.clone());

            // Sequencer namespace is defined by the chain id of attached sequencer node
            // which can be fetched from any block header.
            let sequencer_namespace = {
                let client = sequencer_client_pool
                    .get()
                    .await
                    .wrap_err("failed to get a sequencer client from the pool")?;
                get_sequencer_namespace(client)
                    .await
                    .wrap_err("failed to get sequencer namespace")?
            };
            info!(
                celestia_namespace = %Base64Display::new(sequencer_namespace.as_bytes(), &STANDARD),
                sequencer_chain_id = %cfg.chain_id,
                "celestia namespace derived from sequencer chain id",
            );

            let celestia_config = CelestiaReaderConfig {
                node_url: cfg.celestia_node_url,
                bearer_token: Some(cfg.celestia_bearer_token),
                poll_interval: std::time::Duration::from_secs(3),
            };
            let da_reader = data_availability::Reader::new(
                cfg.initial_da_block_height,
                firm_commit_height,
                celestia_config,
                executor_tx.clone(),
                block_verifier,
                sequencer_namespace,
                celestia_client::blob_space::celestia_namespace_v0_from_hashed_bytes(
                    cfg.chain_id.as_ref(),
                ),
                shutdown_rx,
                sync_done_tx,
            )
            .await
            .wrap_err("failed constructing data availability reader")?;
            data_availability_reader = Some(da_reader);
            shutdown_channels.insert(Self::DATA_AVAILABILITY, shutdown_tx);

            da_sync_done = sync_done_rx.fuse();
        };

        Ok(Self {
            sequencer_reader,
            data_availability_reader,
            sequencer_client_pool,
            shutdown_channels,
            signals,
            seq_sync_done,
            da_sync_done,
            tasks,
            execution_commit_level: cfg.execution_commit_level,
        })
    }

    pub async fn run_until_stopped(mut self) {
        use futures::future::FusedFuture as _;

        info!("starting conductor run loop");
        info!("seq sync status: {:?}", self.seq_sync_done.is_terminated());
        info!("da sync status: {:?}", self.da_sync_done.is_terminated());

        if self.execution_commit_level.is_soft_only() {
            info!("starting sequencer reader");
            if let Some(sequencer_reader) = self.sequencer_reader.take() {
                self.tasks
                    .spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
            }
        } else {
            info!("starting data availability reader");
            if let Some(data_availability_reader) = self.data_availability_reader.take() {
                self.tasks.spawn(
                    Self::DATA_AVAILABILITY,
                    data_availability_reader.run_until_stopped(),
                );
            }
        }

        loop {
            select! {
                // FIXME: The bias should only be on the signal channels. The two handlers should have the same bias.
                biased;

                _ = self.signals.stop_rx.changed() => {
                    info!("shutting down conductor");
                    break;
                }

                _ = self.signals.reload_rx.changed() => {
                    info!("reloading is currently not implemented");
                }

                // Start the sequencer reader
                res = &mut self.da_sync_done, if !self.da_sync_done.is_terminated() => {
                    match res {
                        Ok(()) => info!("received sync-complete signal from da reader"),
                        Err(e) => {
                            let error = &e as &(dyn std::error::Error + 'static);
                            warn!(error, "da sync-complete channel failed prematurely");
                        }
                    }
                    if let Some(sequencer_reader) = self.sequencer_reader.take() {
                        info!("starting sequencer reader");
                        self.tasks.spawn(
                            Self::SEQUENCER,
                            sequencer_reader.run_until_stopped(),
                        );
                    }
                }

                Some((name, res)) = self.tasks.join_next() => {
                    match res {
                        Ok(Ok(())) => error!(task.name = name, "task exited unexpectedly, shutting down"),
                        Ok(Err(e)) => {
                            let error: &(dyn std::error::Error + 'static) = e.as_ref();
                            error!(task.name = name, error, "task exited with error; shutting down");
                        }
                        Err(e) => {
                            let error = &e as &(dyn std::error::Error + 'static);
                            error!(task.name = name, error, "task failed; shutting down");
                        }
                    }
                }
            }
        }

        info!("shutting down conductor");
        self.shutdown().await;
    }

    async fn shutdown(self) {
        info!("sending shutdown command to all tasks");
        for (_, channel) in self.shutdown_channels {
            let _ = channel.send(());
        }

        self.sequencer_client_pool.close();

        info!("waiting 5 seconds for all tasks to shut down");
        // put the tasks into an Rc to make them 'static so they can run on a local set
        let mut tasks = Rc::new(self.tasks);
        let local_set = LocalSet::new();
        local_set
            .run_until(async {
                let mut tasks = tasks.clone();
                let _ = timeout(
                    Duration::from_secs(5),
                    spawn_local(async move {
                        while let Some((name, res)) = Rc::get_mut(&mut tasks)
                            .expect(
                                "only one Rc to the conductor tasks should exist; this is a bug",
                            )
                            .join_next()
                            .await
                        {
                            match res {
                                Ok(Ok(())) => info!(task.name = name, "task exited normally"),
                                Ok(Err(e)) => {
                                    let error: &(dyn std::error::Error + 'static) = e.as_ref();
                                    error!(task.name = name, error, "task exited with error");
                                }
                                Err(e) => {
                                    let error = &e as &(dyn std::error::Error + 'static);
                                    error!(task.name = name, error, "task failed");
                                }
                            }
                        }
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

/// Get the sequencer namespace from the latest sequencer block.
async fn get_sequencer_namespace(
    client: deadpool::managed::Object<ClientProvider>,
) -> eyre::Result<Namespace> {
    use sequencer_client::SequencerClientExt as _;

    let retry_config = tryhard::RetryFutureConfig::new(10)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32,
             next_delay: Option<Duration>,
             error: &sequencer_client::extension_trait::Error| {
                let error = error.clone();
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                async move {
                    let error = &error as &(dyn std::error::Error + 'static);
                    warn!(
                        attempt,
                        wait_duration,
                        error,
                        "attempt to grab sequencer block failed; retrying after backoff",
                    );
                }
            },
        );

    let block = tryhard::retry_fn(|| client.latest_sequencer_block())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get block from sequencer after 10 attempts")?;

    let chain_id = block.into_raw().header.chain_id;

    Ok(celestia_client::blob_space::celestia_namespace_v0_from_hashed_bytes(chain_id.as_bytes()))
}
