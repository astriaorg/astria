use std::{
    collections::HashMap,
    net::SocketAddr,
};

use astria_core::sequencer::v1::transaction::action::SequenceAction;
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
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

use crate::{
    api::{
        self,
    },
    composer_status::ComposerStatus,
    executor,
    executor::{
        Executor,
        ExecutorHandle,
    },
    geth_collector,
    geth_collector::GethCollector,
    grpc_collector::GrpcCollector,
    rollup::Rollup,
    Config,
};

/// `Composer` is a service responsible for spinning up `GethCollectors` which are responsible
/// for fetching pending transactions submitted to the rollup Geth nodes and then passing them
/// downstream for the executor to process. Thus, a composer can have multiple collectors running
/// at the same time funneling data from multiple rollup nodes.
pub struct Composer {
    /// `ApiServerLocalAddr` is the address of the API Server
    api_server_addr: SocketAddr,
    /// `ExecutorHandle` to communicate SequenceActions to the Executor
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: ExecutorHandle,
    /// `GethCollectorStatuses` The collection of the geth collector statuses.
    geth_collector_statuses: HashMap<String, watch::Receiver<geth_collector::Status>>,
    /// `GethCollectorTasks` is the set of tasks tracking if the geth collectors are still running.
    geth_collector_tasks: JoinMap<String, eyre::Result<()>>,
    /// `ComposerTasks` is the set of tasks tracking if the composer is still running.
    /// It mainly consists of the API server, executor and grpc collector.
    composer_tasks: JoinMap<String, eyre::Result<()>>,
    /// `Rollups` The map of chain ID to the URLs to which geth collectors should connect.
    rollups: HashMap<String, String>,
    /// `GrpcCollectorAddr` is the address of the gRPC server of the gRPC collector.
    grpc_collector_addr: SocketAddr,
}

impl Composer {
    const API_SERVER: &'static str = "api_server";
    const EXECUTOR: &'static str = "executor";
    const GRPC_COLLECTOR: &'static str = "grpc_collector";

    /// Constructs a new Composer service from config.
    ///
    /// # Errors
    ///
    /// An error is returned if the composer fails to be initialized.
    /// See `[Composer::from_config]` for its error scenarios.
    pub async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let (composer_status_sender, _) = watch::channel(ComposerStatus::default());

        let (executor, sequence_action_tx) = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
        .wrap_err("executor construction from config failed")?;

        let executor_status = executor.subscribe();

        let executor_handle = ExecutorHandle {
            sequence_action_tx: sequence_action_tx.clone(),
        };

        let grpc_collector_listener = TcpListener::bind(cfg.grpc_collector_addr).await?;
        let grpc_collector_addr = grpc_collector_listener.local_addr()?;
        let grpc_collector = GrpcCollector::new(grpc_collector_listener, executor_handle.clone());

        let api_server = api::start(cfg.api_listen_addr, composer_status_sender.subscribe());
        let api_server_addr = api_server.local_addr();
        info!(
            listen_addr = %api_server_addr,
            "API server listening"
        );

        // spin up composer tasks
        let mut composer_tasks = JoinMap::new();

        // spin up the api server
        composer_tasks.spawn(Self::API_SERVER.to_string(), async move {
            api_server.await.wrap_err("api server ended unexpectedly")
        });

        // spin up the executor
        composer_tasks.spawn(Self::EXECUTOR.to_string(), executor.run_until_stopped());

        // spin up the grpc collector
        composer_tasks.spawn(
            Self::GRPC_COLLECTOR.to_string(),
            grpc_collector.run_until_stopped(),
        );

        // wait for executor
        wait_for_executor(executor_status.clone()).await?;
        composer_status_sender.send_modify(|status| {
            status.set_executor_connected(true);
        });

        let rollups = cfg
            .rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <rollup_name>::<url> pairs as rollups")?;

        let mut geth_collectors = rollups
            .iter()
            .map(|(rollup_name, url)| {
                let collector = GethCollector::new(
                    rollup_name.clone(),
                    url.clone(),
                    sequence_action_tx.clone(),
                );
                (rollup_name.clone(), collector)
            })
            .collect::<HashMap<_, _>>();
        let geth_collector_statuses: HashMap<String, watch::Receiver<geth_collector::Status>> =
            geth_collectors
                .iter()
                .map(|(rollup_name, collector)| (rollup_name.clone(), collector.subscribe()))
                .collect();

        // spin up geth collectors
        let mut geth_collector_tasks = JoinMap::new();
        for (chain_id, collector) in geth_collectors.drain() {
            geth_collector_tasks.spawn(chain_id, collector.run_until_stopped());
        }
        wait_for_collectors(&geth_collector_statuses).await?;
        composer_status_sender.send_modify(|status| {
            status.set_all_collectors_connected(true);
        });

        Ok(Self {
            api_server_addr,
            executor_handle,
            geth_collector_statuses,
            geth_collector_tasks,
            composer_tasks,
            rollups,
            grpc_collector_addr,
        })
    }

    /// Returns the socket address the api server is served over
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server_addr
    }

    /// Returns the socker address the grpc collector is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    #[must_use]
    pub fn grpc_collector_addr(&self) -> SocketAddr {
        self.grpc_collector_addr
    }

    /// Runs the composer.
    ///
    /// # Errors
    /// It errors out if the API Server, Executor or any of the Geth Collectors fail to start.
    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            mut composer_tasks,
            executor_handle,
            mut geth_collector_tasks,
            rollups,
            mut geth_collector_statuses,
            ..
        } = self;

        loop {
            tokio::select!(
            Some((task, err)) = composer_tasks.join_next() => {
                report_exit(format!("composer task: {task}").as_str(), err);
                return Ok(());
            },
            Some((rollup, collector_exit)) = geth_collector_tasks.join_next() => {
                // TODO - do we really need to restart the geth collector?
                reconnect_exited_collector(
                    &mut geth_collector_statuses,
                    &mut geth_collector_tasks,
                    executor_handle.sequence_action_tx.clone(),
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
    collector_statuses: &HashMap<String, watch::Receiver<geth_collector::Status>>,
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
                match status.wait_for(geth_collector::Status::is_connected).await {
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

fn reconnect_exited_collector(
    collector_statuses: &mut HashMap<String, watch::Receiver<geth_collector::Status>>,
    collector_tasks: &mut JoinMap<String, eyre::Result<()>>,
    serialized_rolup_transactions_tx: Sender<SequenceAction>,
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

    let collector = GethCollector::new(
        rollup.clone(),
        url.clone(),
        serialized_rolup_transactions_tx,
    );
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

    use astria_core::sequencer::v1::{
        asset::default_native_asset_id,
        transaction::action::SequenceAction,
        RollupId,
    };
    use ethers::types::Transaction;
    use tokio_util::task::JoinMap;

    use crate::{
        geth_collector,
        geth_collector::GethCollector,
    };

    /// This tests the `reconnect_exited_collector` handler.
    #[tokio::test]
    async fn collector_is_reconnected_after_exit() {
        let mock_geth = test_utils::mock::Geth::spawn().await;
        let rollup_name = "test".to_string();
        let rollup_url = format!("ws://{}", mock_geth.local_addr());
        let rollups = HashMap::from([(rollup_name.clone(), rollup_url.clone())]);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        let mut collector_tasks = JoinMap::new();
        let collector = GethCollector::new(rollup_name.clone(), rollup_url.clone(), tx.clone());
        let mut status = collector.subscribe();
        collector_tasks.spawn(rollup_name.clone(), collector.run_until_stopped());
        status
            .wait_for(geth_collector::Status::is_connected)
            .await
            .unwrap();
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
            tx.clone(),
            &rollups,
            rollup_name.clone(),
            exit_result,
        );

        assert!(collector_tasks.contains_key(&rollup_name));
        statuses
            .get_mut(&rollup_name)
            .unwrap()
            .wait_for(geth_collector::Status::is_connected)
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
