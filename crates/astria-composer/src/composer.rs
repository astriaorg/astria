use std::{
    collections::HashMap,
    net::SocketAddr,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use itertools::Itertools as _;
use tokio::{
    io,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::watch,
    task::{
        JoinError,
        JoinHandle,
    },
    time::timeout,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    api::{
        self,
        ApiServer,
    },
    collectors,
    collectors::geth,
    composer,
    executor,
    executor::Executor,
    grpc,
    grpc::GrpcServer,
    rollup::Rollup,
    Config,
};

const API_SERVER_SHUTDOWN_DURATION: Duration = Duration::from_secs(2);
const GRPC_SERVER_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);
const EXECUTOR_SHUTDOWN_DURATION: Duration = Duration::from_secs(17);
const GETH_COLLECTOR_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

/// `Composer` is a service responsible for spinning up `GethCollectors` which are responsible
/// for fetching pending transactions submitted to the rollup Geth nodes and then passing them
/// downstream for the executor to process. Thus, a composer can have multiple collectors running
/// at the same time funneling data from multiple rollup nodes.
pub struct Composer {
    /// used for monitoring the status of the Composer service.
    api_server: ApiServer,
    /// used to announce the current status of the Composer for other
    /// modules in the crate to use.
    composer_status_sender: watch::Sender<composer::Status>,
    /// used to forward transactions received from rollups to the Executor.
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: executor::Handle,
    /// responsible for signing and submitting sequencer transactions.
    /// The sequencer transactions are received from various collectors.
    executor: Executor,
    /// The collection of geth collectors and their rollup names.
    geth_collectors: HashMap<String, collectors::Geth>,
    /// The collection of the status of each geth collector.
    geth_collector_statuses: HashMap<String, watch::Receiver<collectors::geth::Status>>,
    /// The set of tasks tracking if the geth collectors are still running and to receive
    /// the final result of each geth collector.
    geth_collector_tasks: JoinMap<String, eyre::Result<()>>,
    /// The map of chain ID to the URLs to which geth collectors should connect.
    rollups: HashMap<String, String>,
    /// The gRPC server that listens for incoming requests from the collectors via the
    /// GrpcCollector service. It also exposes a health service.
    grpc_server: GrpcServer,
    /// Used to signal the Composer to shut down.
    shutdown_token: CancellationToken,
}

/// Announces the current status of the Composer for other modules in the crate to use
#[derive(Debug, Default)]
pub(super) struct Status {
    all_collectors_connected: bool,
    executor_connected: bool,
}

impl Status {
    pub(super) fn is_ready(&self) -> bool {
        self.all_collectors_connected && self.executor_connected
    }

    pub(super) fn set_all_collectors_connected(&mut self, connected: bool) {
        self.all_collectors_connected = connected;
    }

    pub(super) fn set_executor_connected(&mut self, connected: bool) {
        self.executor_connected = connected;
    }
}

impl Composer {
    /// Constructs a new Composer service from config.
    ///
    /// # Errors
    ///
    /// An error is returned if the composer fails to be initialized.
    /// See `[from_config]` for its error scenarios.
    pub async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let (composer_status_sender, _) = watch::channel(Status::default());
        let shutdown_token = CancellationToken::new();

        let (executor, executor_handle) = executor::Builder {
            sequencer_url: cfg.sequencer_url.clone(),
            private_key: cfg.private_key.clone(),
            block_time: cfg.block_time_ms,
            max_bytes_per_bundle: cfg.max_bytes_per_bundle,
            shutdown_token: shutdown_token.clone(),
        }
        .build()
        .wrap_err("executor construction from config failed")?;

        let grpc_server = grpc::Builder {
            grpc_addr: cfg.grpc_addr,
            executor: executor_handle.clone(),
            shutdown_token: shutdown_token.clone(),
        }
        .build()
        .await
        .wrap_err("grpc server construction from config failed")?;

        info!(
            listen_addr = %grpc_server.local_addr().wrap_err("grpc server listener not bound")?,
            "gRPC server listening"
        );

        let api_server = api::start(cfg.api_listen_addr, composer_status_sender.subscribe());

        info!(
            listen_addr = %api_server.local_addr(),
            "API server listening"
        );

        let rollups = cfg
            .rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <rollup_name>::<url> pairs as rollups")?;

        let geth_collectors = rollups
            .iter()
            .map(|(rollup_name, url)| {
                let collector = geth::Builder {
                    chain_name: rollup_name.clone(),
                    url: url.clone(),
                    executor_handle: executor_handle.clone(),
                    shutdown_token: shutdown_token.clone(),
                }
                .build();
                (rollup_name.clone(), collector)
            })
            .collect::<HashMap<_, _>>();
        let geth_collector_statuses: HashMap<String, watch::Receiver<geth::Status>> =
            geth_collectors
                .iter()
                .map(|(rollup_name, collector)| (rollup_name.clone(), collector.subscribe()))
                .collect();

        Ok(Self {
            api_server,
            composer_status_sender,
            executor_handle,
            executor,
            rollups,
            geth_collectors,
            geth_collector_statuses,
            geth_collector_tasks: JoinMap::new(),
            grpc_server,
            shutdown_token,
        })
    }

    /// Returns the socket address the api server is served over
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Returns the socker address the grpc server is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub fn grpc_local_addr(&self) -> io::Result<SocketAddr> {
        self.grpc_server.local_addr()
    }

    /// Runs the composer.
    ///
    /// # Errors
    /// It errors out if the API Server, Executor or any of the Geth Collectors fail to start.
    ///
    /// # Panics
    /// It panics if the Composer cannot set the SIGTERM listener.
    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            api_server,
            mut composer_status_sender,
            executor,
            executor_handle,
            mut geth_collector_tasks,
            mut geth_collectors,
            rollups,
            mut geth_collector_statuses,
            grpc_server,
            shutdown_token,
        } = self;

        // we need the API server to shutdown at the end, since it is used by k8s
        // to report the liveness of the service
        let api_server_shutdown_token = CancellationToken::new();

        let api_server_task_shutdown_token = api_server_shutdown_token.clone();
        // run the api server
        let mut api_task = tokio::spawn(async move {
            api_server
                .with_graceful_shutdown(api_server_task_shutdown_token.cancelled())
                .await
                .wrap_err("api server ended unexpectedly")
        });

        // run the collectors and executor
        spawn_geth_collectors(&mut geth_collectors, &mut geth_collector_tasks);

        let executor_status = executor.subscribe().clone();
        let mut executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for collectors and executor to come online
        wait_for_collectors(&geth_collector_statuses, &mut composer_status_sender)
            .await
            .wrap_err("geth collectors failed to become ready")?;
        wait_for_executor(executor_status, &mut composer_status_sender)
            .await
            .wrap_err("executor failed to become ready")?;

        // run the grpc server
        let mut grpc_server_handle = tokio::spawn(async move {
            grpc_server
                .run_until_stopped()
                .await
                .wrap_err("grpc server failed")
        });

        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
        );

        let shutdown_info = loop {
            tokio::select!(
            biased;
            _ = sigterm.recv() => {
                    info!("received SIGTERM; shutting down");
                    break ShutdownInfo {
                        api_server_shutdown_token,
                        composer_shutdown_token: shutdown_token,
                        api_server_task_handle: Some(api_task),
                        executor_task_handle: Some(executor_task),
                        grpc_server_task_handle: Some(grpc_server_handle),
                        geth_collector_tasks,
                    };
            },
            o = &mut api_task => {
                    report_exit("api server unexpectedly ended", o);
                    break ShutdownInfo {
                        api_server_shutdown_token,
                        composer_shutdown_token: shutdown_token,
                        api_server_task_handle: None,
                        executor_task_handle: Some(executor_task),
                        grpc_server_task_handle: Some(grpc_server_handle),
                        geth_collector_tasks,
                    };
            },
            o = &mut executor_task => {
                    report_exit("executor unexpectedly ended", o);
                    break ShutdownInfo {
                        api_server_shutdown_token,
                        composer_shutdown_token: shutdown_token,
                        api_server_task_handle: Some(api_task),
                        executor_task_handle: None,
                        grpc_server_task_handle: Some(grpc_server_handle),
                        geth_collector_tasks,
                    };
            },
            o = &mut grpc_server_handle => {
                    report_exit("grpc server unexpectedly ended", o);
                    break ShutdownInfo {
                        api_server_shutdown_token,
                        composer_shutdown_token: shutdown_token,
                        api_server_task_handle: Some(api_task),
                        executor_task_handle: Some(executor_task),
                        grpc_server_task_handle: None,
                        geth_collector_tasks,
                    };
            },
            Some((rollup, collector_exit)) = geth_collector_tasks.join_next() => {
                   reconnect_exited_collector(
                    &mut geth_collector_statuses,
                    &mut geth_collector_tasks,
                    executor_handle.clone(),
                    &rollups,
                    rollup,
                    collector_exit,
                    shutdown_token.clone()
                );
            });
        };

        shutdown_info.run().await
    }
}

struct ShutdownInfo {
    api_server_shutdown_token: CancellationToken,
    composer_shutdown_token: CancellationToken,
    api_server_task_handle: Option<JoinHandle<eyre::Result<()>>>,
    executor_task_handle: Option<JoinHandle<eyre::Result<()>>>,
    grpc_server_task_handle: Option<JoinHandle<eyre::Result<()>>>,
    geth_collector_tasks: JoinMap<String, eyre::Result<()>>,
}

impl ShutdownInfo {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            composer_shutdown_token,
            api_server_shutdown_token,
            api_server_task_handle,
            executor_task_handle,
            grpc_server_task_handle,
            mut geth_collector_tasks,
        } = self;

        // if the composer is shutting down because of an unexpected shutdown from any one of the
        // components(and not because of a SIGTERM), we need to send the cancel signal to all
        // the other components.
        composer_shutdown_token.cancel();
        // k8s issues SIGKILL in 30s, so we need to make sure that the shutdown happens before 30s.

        // We give executor 17 seconds to shut down. The logic to timeout is in the
        // executor itself. We wait 17s for all the bundles to be drained.
        if let Some(executor_task_handle) = executor_task_handle {
            match tokio::time::timeout(EXECUTOR_SHUTDOWN_DURATION, executor_task_handle)
                .await
                .map(flatten_result)
            {
                Ok(Ok(())) => info!("executor task shut down"),
                Ok(Err(error)) => error!(%error, "executor task shut down with error"),
                Err(error) => error!(%error, "executor task failed to shut down in time"),
            }
        } else {
            info!("executor task was already dead");
        };

        // We give the grpc server 5 seconds to shut down.
        if let Some(grpc_server_task_handle) = grpc_server_task_handle {
            match tokio::time::timeout(GRPC_SERVER_SHUTDOWN_DURATION, grpc_server_task_handle)
                .await
                .map(flatten_result)
            {
                Ok(Ok(())) => info!("grpc server task shut down"),
                Ok(Err(error)) => error!(%error, "grpc server task shut down with error"),
                Err(error) => error!(%error, "grpc server task failed to shut down in time"),
            }
        } else {
            info!("grpc server task was already dead");
        };

        let shutdown_loop = async {
            while let Some((name, res)) = geth_collector_tasks.join_next().await {
                let message = "task shut down";
                match flatten_result(res) {
                    Ok(()) => info!(name, message),
                    Err(error) => error!(name, %error, message),
                }
            }
        };

        // we give 5s to shut down all the other geth collectors. geth collectors shouldn't take
        // too long to shutdown since they just need to unsubscribe to their WSS
        // streams.
        if timeout(GETH_COLLECTOR_SHUTDOWN_DURATION, shutdown_loop)
            .await
            .is_err()
        {
            let tasks = geth_collector_tasks.keys().join(", ");
            warn!(
                tasks = format_args!("[{tasks}]"),
                "aborting all geth collector tasks that have not yet shut down",
            );
            geth_collector_tasks.abort_all();
        } else {
            info!("all geth collector tasks shut down regularly");
        }

        // cancel the api server at the end
        // we give the api server 2s, since it shouldn't be getting too much traffic and should
        // be able to shut down faster.
        api_server_shutdown_token.cancel();
        if let Some(api_server_task_handle) = api_server_task_handle {
            match tokio::time::timeout(API_SERVER_SHUTDOWN_DURATION, api_server_task_handle)
                .await
                .map(flatten_result)
            {
                Ok(Ok(())) => info!("api server task shut down"),
                Ok(Err(error)) => error!(%error, "api server task shutdown with error"),
                Err(error) => error!(%error, "api server task failed to shutdown in time"),
            }
        } else {
            info!("api server task was already dead");
        };

        Ok(())
    }
}

fn spawn_geth_collectors(
    geth_collectors: &mut HashMap<String, collectors::Geth>,
    geth_collector_tasks: &mut JoinMap<String, eyre::Result<()>>,
) {
    for (chain_id, collector) in geth_collectors.drain() {
        geth_collector_tasks.spawn(chain_id, collector.run_until_stopped());
    }
}

async fn wait_for_executor(
    mut executor_status: watch::Receiver<executor::Status>,
    composer_status_sender: &mut watch::Sender<composer::Status>,
) -> eyre::Result<()> {
    executor_status
        .wait_for(executor::Status::is_connected)
        .await
        .wrap_err("executor failed while waiting for it to become ready")?;

    composer_status_sender.send_modify(|status| {
        status.set_executor_connected(true);
    });

    Ok(())
}

/// Waits for all collectors to come online.
async fn wait_for_collectors(
    collector_statuses: &HashMap<String, watch::Receiver<collectors::geth::Status>>,
    composer_status_sender: &mut watch::Sender<composer::Status>,
) -> eyre::Result<()> {
    use futures::{
        future::FutureExt as _,
        stream::{
            FuturesUnordered,
            StreamExt as _,
        },
    };
    let mut statuses = collector_statuses
        .iter()
        .map(|(chain_id, status)| {
            let mut status = status.clone();
            async move {
                match status
                    .wait_for(collectors::geth::Status::is_connected)
                    .await
                {
                    // `wait_for` returns a reference to status; throw it
                    // away because this future cannot return a reference to
                    // a stack local object.
                    Ok(_) => Ok(()),
                    // if a collector fails while waiting for its status, this
                    // will return an error
                    Err(e) => Err(e),
                }
            }
            .map(|fut| (chain_id.clone(), fut))
        })
        .collect::<FuturesUnordered<_>>();
    while let Some((chain_id, maybe_err)) = statuses.next().await {
        if let Err(e) = maybe_err {
            return Err(e).wrap_err_with(|| {
                format!(
                    "collector for chain ID {chain_id} failed while waiting for it to become ready"
                )
            });
        }
    }

    composer_status_sender.send_modify(|status| {
        status.set_all_collectors_connected(true);
    });

    Ok(())
}

pub(super) fn reconnect_exited_collector(
    collector_statuses: &mut HashMap<String, watch::Receiver<collectors::geth::Status>>,
    collector_tasks: &mut JoinMap<String, eyre::Result<()>>,
    executor_handle: executor::Handle,
    rollups: &HashMap<String, String>,
    rollup: String,
    exit_result: Result<eyre::Result<()>, JoinError>,
    shutdown_token: CancellationToken,
) {
    report_exit("collector", exit_result);
    let Some(url) = rollups.get(&rollup) else {
        error!(
            "rollup should have had an entry in the rollup->url map but doesn't; not reconnecting \
             it"
        );
        return;
    };

    let collector = geth::Builder {
        chain_name: rollup.clone(),
        url: url.clone(),
        executor_handle,
        shutdown_token,
    }
    .build();
    collector_statuses.insert(rollup.clone(), collector.subscribe());
    collector_tasks.spawn(rollup, collector.run_until_stopped());
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => info!(task = task_name, "task exited successfully"),
        Ok(Err(error)) => {
            error!(%error, task = task_name, "task returned with error");
        }
        Err(error) => {
            error!(%error, task = task_name, "task failed to complete");
        }
    }
}

pub(crate) fn flatten_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
