use std::{
    collections::HashMap,
    net::SocketAddr,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    io,
    net::TcpListener,
    sync::{
        mpsc::Sender,
        watch,
    },
    task::JoinError,
};
use tokio_util::task::JoinMap;
use tracing::{
    error,
    info,
};
use crate::{api::{
    self,
    ApiServer,
}, collectors, collectors::Geth, executor, executor::Executor, rollup::Rollup, Config, composer};
use crate::composer_status::ComposerStatus;
use crate::grpc_collector::GrpcCollector;

/// `Composer` is a service responsible for spinning up `GethCollectors` which are responsible
/// for fetching pending transactions submitted to the rollup Geth nodes and then passing them
/// downstream for the executor to process. Thus, a composer can have multiple collectors running
/// at the same time funneling data from multiple rollup nodes.
pub struct Composer {
    /// `ApiServer` is used for monitoring status of the Composer service.
    api_server: ApiServer,
    /// `ComposerStatusSender` is used to announce the current status of the Composer for other
    /// modules in the crate to use.
    composer_status_sender: watch::Sender<composer::Status>,
    /// `ExecutorHandle` to communicate SequenceActions to the Executor
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: executor::Handle,
    /// `Executor` is responsible for signing and submitting sequencer transactions
    /// The sequencer transactions are received from various collectors.
    executor: Executor,
    /// `GethCollectors` is the collection of geth collectors and their rollup names.
    geth_collectors: HashMap<String, collectors::Geth>,
    /// `GethCollectorStatuses` The collection of the geth collector statuses.
    geth_collector_statuses: HashMap<String, watch::Receiver<collectors::Status>>,
    /// `GethCollectorTasks` is the set of tasks tracking if the geth collectors are still running.
    geth_collector_tasks: JoinMap<String, eyre::Result<()>>,
    /// `Rollups` The map of chain ID to the URLs to which geth collectors should connect.
    rollups: HashMap<String, String>,
    /// `GrpcCollector` is a gRPC server to which users can send sequence actions
    grpc_collector: GrpcCollector,
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
    /// See `[Composer::from_config]` for its error scenarios.
    pub async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let (composer_status_sender, _) = watch::channel(Status::default());
        let (executor, executor_handle) = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
            .wrap_err("executor construction from config failed")?;


        let grpc_collector_listener = TcpListener::bind(cfg.grpc_collector_addr).await?;
        let grpc_collector = GrpcCollector::new(grpc_collector_listener, executor_handle.clone());

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
                let collector =
                    Geth::new(rollup_name.clone(), url.clone(), executor_handle.clone());
                (rollup_name.clone(), collector)
            })
            .collect::<HashMap<_, _>>();
        let geth_collector_statuses: HashMap<String, watch::Receiver<collectors::geth::Status>> =
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
            grpc_collector,
        })
    }

    /// Returns the socket address the api server is served over
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Returns the socker address the grpc collector is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub fn grpc_collector_local_addr(&self) -> io::Result<SocketAddr> {
        self.grpc_collector.grpc_collector_local_addr()
    }

    /// Runs the composer.
    ///
    /// # Errors
    /// It errors out if the API Server, Executor or any of the Geth Collectors fail to start.
    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            api_server,
            composer_status_sender,
            executor,
            executor_handle,
            mut geth_collector_tasks,
            mut geth_collectors,
            rollups,
            mut geth_collector_statuses,
            grpc_collector,
        } = self;

        // run the api server
        let mut api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });

        // run the collectors and executor
        for (chain_id, collector) in geth_collectors.drain() {
            geth_collector_tasks.spawn(chain_id, collector.run_until_stopped());
        }
        let executor_status = executor.subscribe().clone();
        let mut executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for collectors and executor to come online
        wait_for_collectors(&geth_collector_statuses).await?;
        composer_status_sender.send_modify(|status| {
            status.set_all_collectors_connected(true);
        });
        wait_for_executor(executor_status).await?;
        composer_status_sender.send_modify(|status| {
            status.set_executor_connected(true);
        });

        // run the grpc server
        let mut grpc_server_handle = tokio::spawn(async move {
            grpc_collector
                .run_until_stopped()
                .await
                .wrap_err("grpc server failed")
        });

        loop {
            tokio::select!(
            o = &mut api_task => {
                    report_exit("api server unexpectedly ended", o);
                    return Ok(());
            },
            o = &mut executor_task => {
                    report_exit("executor unexpectedly ended", o);
                    return Ok(());
            },
            o = &mut grpc_server_handle => {
                    report_exit("grpc server unexpectedly ended", o);
                    return Ok(());
            },
            Some((rollup, collector_exit)) = geth_collector_tasks.join_next() => {
                   reconnect_exited_collector(
                    &mut geth_collector_statuses,
                    &mut geth_collector_tasks,
                    executor_handle.clone(),
                    &rollups,
                    rollup,
                    collector_exit,
                );
            });
        }
    }
}

async fn wait_for_executor(
    mut executor_status: watch::Receiver<executor::Status>,
) -> eyre::Result<()> {
    executor_status
        .wait_for(executor::Status::is_connected)
        .await
        .wrap_err("executor failed while waiting for it to become ready")?;

    Ok(())
}

/// Waits for all collectors to come online.
async fn wait_for_collectors(
    collector_statuses: &HashMap<String, watch::Receiver<collectors::geth::Status>>,
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

    Ok(())
}

pub(super) fn reconnect_exited_collector(
    collector_statuses: &mut HashMap<String, watch::Receiver<collectors::geth::Status>>,
    collector_tasks: &mut JoinMap<String, eyre::Result<()>>,
    executor_handle: executor::Handle,
    rollups: &HashMap<String, String>,
    rollup: String,
    exit_result: Result<eyre::Result<()>, JoinError>,
) {
    report_exit("collector", exit_result);
    let Some(url) = rollups.get(&rollup) else {
        error!(
            "rollup should have had an entry in the rollup->url map but doesn't; not reconnecting \
             it"
        );
        return;
    };

    let collector = Geth::new(rollup.clone(), url.clone(), executor_handle);
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ethers::types::Transaction;
    use tokio_util::task::JoinMap;
    use astria_core::sequencer::v1::asset::default_native_asset_id;
    use astria_core::sequencer::v1::RollupId;
    use astria_core::sequencer::v1::transaction::action::SequenceAction;
    use crate::{collectors, executor};

    /// This tests the `reconnect_exited_collector` handler.
    #[tokio::test]
    async fn collector_is_reconnected_after_exit() {
        let mock_geth = test_utils::mock::Geth::spawn().await;
        let rollup_name = "test".to_string();
        let rollup_url = format!("ws://{}", mock_geth.local_addr());
        let rollups = HashMap::from([(rollup_name.clone(), rollup_url.clone())]);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let executor_handle = executor::Handle::new(tx.clone());

        let mut collector_tasks = JoinMap::new();
        let collector = collectors::Geth::new(rollup_name.clone(), rollup_url.clone(), executor_handle.clone());
        let mut status = collector.subscribe();
        collector_tasks.spawn(rollup_name.clone(), collector.run_until_stopped());
        status.wait_for(collectors::Status::is_connected).await.unwrap();
        let rollup_tx = Transaction::default();
        let expected_seq_action = SequenceAction {
            rollup_id: RollupId::from_unhashed_bytes(&rollup_name),
            data: Transaction::default().rlp().to_vec(),
            fee_asset_id: default_native_asset_id(),
        };
        let _ = mock_geth.push_tx(rollup_tx.clone()).unwrap();
        let collector_tx = rx.recv().await.unwrap();

        assert_eq!(
            RollupId::from_unhashed_bytes(&rollup_name),
            collector_tx.rollup_id,
        );
        assert_eq!(expected_seq_action.data, collector_tx.data);

        let _ = mock_geth.abort().unwrap();

        let (exited_rollup_name, exit_result) = collector_tasks.join_next().await.unwrap();
        assert_eq!(exited_rollup_name, rollup_name);
        assert!(collector_tasks.is_empty());

        // after aborting pushing a new tx to subscribers should fail as there are no broadcast
        // receivers
        assert!(mock_geth.push_tx(Transaction::default()).is_err());

        let mut statuses = HashMap::new();
        super::reconnect_exited_collector(
            &mut statuses,
            &mut collector_tasks,
            executor_handle.clone(),
            &rollups,
            rollup_name.clone(),
            exit_result,
        );

        assert!(collector_tasks.contains_key(&rollup_name));
        statuses
            .get_mut(&rollup_name)
            .unwrap()
            .wait_for(collectors::Status::is_connected)
            .await
            .unwrap();
        let _ = mock_geth.push_tx(rollup_tx).unwrap();
        let collector_tx = rx.recv().await.unwrap();

        assert_eq!(
            RollupId::from_unhashed_bytes(&rollup_name),
            collector_tx.rollup_id,
        );
        assert_eq!(expected_seq_action.data, collector_tx.data);
    }
}
