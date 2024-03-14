use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
};

use astria_core::{
    generated::composer::v1alpha1::grpc_collector_service_server::GrpcCollectorServiceServer,
    sequencer::v1::transaction::action::SequenceAction,
};
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
        ApiServer,
    },
    composer_service::ExecutorHandle,
    searcher::{
        executor,
        executor::Executor,
        geth_collector,
        geth_collector::GethCollector,
        Status,
    },
    Config,
};

/// Composer is a service responsible for submitting transactions to the Astria
/// Shared Sequencer.
pub struct Composer {
    /// `ApiServer` is used for monitoring status of the Composer service.
    api_server: ApiServer,
    /// `Searcher` establishes connections to individual rollup nodes, receiving
    /// pending transactions from them and wraps them as sequencer transactions
    /// for submission.
    // searcher: Searcher,
    searcher_status_sender: watch::Sender<Status>,
    /// `ExecutorHandle` to communicate SequenceActions to the Executor
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: ExecutorHandle,
    /// `Executor` is responsible for signing and submitting sequencer transactions
    /// The sequencer transactions are received from various collectors.
    executor: Executor,
    /// `GrpcCollectorListener` is the tcp connection on which the gRPC collector is running
    grpc_collector_listener: TcpListener,

    // The collection of collectors and their rollup names.
    geth_collectors: HashMap<String, GethCollector>,
    // The collection of the collector statuses.
    collector_statuses: HashMap<String, watch::Receiver<geth_collector::Status>>,
    // The map of chain ID to the URLs to which collectors should connect.
    rollups: HashMap<String, String>,
    // The set of tasks tracking if the geth collectors are still running.
    collector_tasks: JoinMap<String, eyre::Result<()>>,
}

impl Composer {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// An error is returned if the searcher fails to be initialized.
    /// See `[Searcher::from_config]` for its error scenarios.
    pub async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let (serialized_rollup_transactions_tx, serialized_rollup_transactions_rx) =
            tokio::sync::mpsc::channel(256);
        let (searcher_status_sender, _) = watch::channel(Status::default());

        let executor_handle = ExecutorHandle {
            sequence_action_tx: serialized_rollup_transactions_tx.clone(),
        };

        let executor = Executor::new(
            &cfg.sequencer_url,
            &cfg.private_key,
            serialized_rollup_transactions_rx,
            cfg.block_time_ms,
            cfg.max_bytes_per_bundle,
        )
        .wrap_err("executor construction from config failed")?;

        let grpc_collector_listener = TcpListener::bind(cfg.grpc_collector_addr).await?;

        let api_server = api::start(cfg.api_listen_addr, searcher_status_sender.subscribe());
        info!(
            listen_addr = %api_server.local_addr(),
            "API server listening"
        );

        let rollups = cfg
            .rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| {
                crate::searcher::rollup::Rollup::parse(s)
                    .map(crate::searcher::rollup::Rollup::into_parts)
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <rollup_name>::<url> pairs as rollups")?;

        let geth_collectors = rollups
            .iter()
            .map(|(rollup_name, url)| {
                let collector = GethCollector::new(
                    rollup_name.clone(),
                    url.clone(),
                    serialized_rollup_transactions_tx.clone(),
                );
                (rollup_name.clone(), collector)
            })
            .collect::<HashMap<_, _>>();
        let collector_statuses: HashMap<String, watch::Receiver<geth_collector::Status>> =
            geth_collectors
                .iter()
                .map(|(rollup_name, collector)| (rollup_name.clone(), collector.subscribe()))
                .collect();

        Ok(Self {
            api_server,
            searcher_status_sender,
            executor_handle,
            executor,
            grpc_collector_listener,
            rollups,
            geth_collectors,
            collector_statuses,
            collector_tasks: JoinMap::new(),
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
        self.grpc_collector_listener.local_addr()
    }

    /// Runs the composer.
    ///
    /// Currently only exits if the api server or searcher stop unexpectedly.
    /// # Errors
    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        let Self {
            api_server,
            searcher_status_sender,
            grpc_collector_listener,
            executor,
            executor_handle,
            mut collector_tasks,
            mut geth_collectors,
            rollups,
            mut collector_statuses,
        } = self;

        // run the api server
        let mut api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });

        // run the collectors and executor
        for (chain_id, collector) in geth_collectors.drain() {
            collector_tasks.spawn(chain_id, collector.run_until_stopped());
        }
        let executor_status = executor.subscribe().clone();
        let mut executor_task = tokio::spawn(executor.run_until_stopped());

        // wait for collectors and executor to come online
        wait_for_collectors(&collector_statuses).await?;
        searcher_status_sender.send_modify(|status| {
            status.all_collectors_connected = true;
        });
        wait_for_executor(executor_status).await?;
        searcher_status_sender.send_modify(|status| {
            status.executor_connected = true;
        });

        // run the grpc collector
        let composer_service = GrpcCollectorServiceServer::new(executor_handle.clone());
        let grpc_server = tonic::transport::Server::builder().add_service(composer_service);
        let mut grpc_server_handler = tokio::spawn(async move {
            grpc_server
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(
                    grpc_collector_listener,
                ))
                .await
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
            exit_error = &mut grpc_server_handler => {
                    match exit_error {
                        Ok(Ok(())) => info!("grpc server exited unexpectedly; reconnecting"),
                        Ok(Err(error)) => {
                            error!(%error, "grpc server exit with error; reconnecting");
                        }
                        Err(error) => {
                            error!(%error, "grpc server task failed; reconnecting");
                        }
                    }
                    return Ok(());
            },
            Some((rollup, collector_exit)) = collector_tasks.join_next() => {
                reconnect_exited_collector(
                    &mut collector_statuses,
                    &mut collector_tasks,
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

    use crate::searcher::geth_collector::{
        GethCollector,
        Status,
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
        status.wait_for(Status::is_connected).await.unwrap();
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
            .wait_for(Status::is_connected)
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
