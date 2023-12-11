use std::collections::HashMap;

use astria_core::sequencer::v1alpha1::transaction::{
    action::SequenceAction,
    Action,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            Receiver,
            Sender,
        },
        watch,
    },
    task::{
        JoinError,
        JoinSet,
    },
};
use tokio_util::task::JoinMap;
use tracing::{
    error,
    instrument,
    warn,
};

use crate::Config;

mod collector;
mod executor;
mod rollup;

use collector::Collector;
use executor::Executor;

/// A Searcher collates transactions from multiple rollups and bundles them into
/// Astria sequencer transactions that are then passed on to the
/// Shared Sequencer. The rollup transactions that make up these sequencer transactions
/// have have the property of atomic inclusion, i.e. if they are submitted to the
/// sequencer, all of them are going to be executed in the same Astria block.
pub(super) struct Searcher {
    // Channel to report the internal status of the searcher to other parts of the system.
    status: watch::Sender<Status>,
    // The collection of collectors and their rollup names.
    collectors: HashMap<String, Collector>,
    // The collection of the collector statuses.
    collector_statuses: HashMap<String, watch::Receiver<collector::Status>>,
    // The rx part of the channel on which the searcher receives transactions from its collectors.
    new_transactions_rx: Receiver<collector::Transaction>,
    // The tx part that collectors use to send transactions to their parent searcher.
    new_transactions_tx: Sender<collector::Transaction>,
    // The map of chain ID to the URLs to which collectors should connect.
    rollups: HashMap<String, String>,
    // The set of tasks tracking if the collectors are still running.
    collector_tasks: JoinMap<String, eyre::Result<()>>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<Vec<Action>>,
    // The Executor object that is responsible for signing and submitting sequencer transactions.
    executor: Option<Executor>,
    // A channel on which to send the `Executor` bundles for attaching a nonce to, sign and submit
    bundle_tx: mpsc::Sender<Vec<Action>>,
    // Channel from which to read the internal status of the executor.
    executor_status: watch::Receiver<executor::Status>,
}

/// Announces the current status of the Searcher for other modules in the crate to use
#[derive(Debug, Default)]
pub(crate) struct Status {
    all_collectors_connected: bool,
    executor_connected: bool,
}

impl Status {
    pub(crate) fn is_ready(&self) -> bool {
        self.all_collectors_connected && self.executor_connected
    }
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// Errors are returned in the following scenarios:
    /// + failed to connect to the eth RPC server;
    /// + failed to construct a sequencer clinet
    pub(super) fn from_config(cfg: &Config) -> eyre::Result<Self> {
        use rollup::Rollup;
        let rollups = cfg
            .rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <rollup_name>::<url> pairs as rollups")?;

        let (new_transactions_tx, new_transactions_rx) = mpsc::channel(256);

        let collectors = rollups
            .iter()
            .map(|(rollup_name, url)| {
                let collector = Collector::new(
                    rollup_name.clone(),
                    url.clone(),
                    new_transactions_tx.clone(),
                );
                (rollup_name.clone(), collector)
            })
            .collect::<HashMap<_, _>>();
        let collector_statuses = collectors
            .iter()
            .map(|(rollup_name, collector)| (rollup_name.clone(), collector.subscribe()))
            .collect();

        let (status, _) = watch::channel(Status::default());

        let (bundle_tx, bundle_rx) = mpsc::channel(256);
        let executor = Executor::new(&cfg.sequencer_url, &cfg.private_key, bundle_rx)
            .wrap_err("executor construction from config failed")?;

        let executor_status = executor.subscribe();

        Ok(Searcher {
            status,
            collectors,
            collector_statuses,
            new_transactions_rx,
            new_transactions_tx,
            collector_tasks: JoinMap::new(),
            conversion_tasks: JoinSet::new(),
            bundle_tx,
            executor_status,
            executor: Some(executor),
            rollups,
        })
    }

    /// Other modules can use this to get notified of changes to the Searcher state
    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    fn bundle_pending_tx(&mut self, tx: collector::Transaction) {
        let collector::Transaction {
            rollup_id,
            inner: rollup_tx,
        } = tx;

        // rollup transaction data serialization is a heavy compute task, so it is spawned
        // on tokio's blocking threadpool
        self.conversion_tasks.spawn_blocking(move || {
            let data = rollup_tx.rlp().to_vec();
            let seq_action = Action::Sequence(SequenceAction {
                rollup_id,
                data,
            });

            vec![seq_action]
        });
    }

    async fn handle_bundle_execution(&self, bundle: Vec<Action>) {
        // send bundle to executor
        if let Err(e) = self.bundle_tx.send(bundle).await {
            error!(
                error.message = %e,
                error.cause_chain = ?e,
                "failed to send bundle to executor",
            );
        }
    }

    /// Starts the searcher and runs it until failure
    ///
    /// # Backpressure
    /// The current implementation suffers from a backpressure problem. See issue #409 for an
    /// in-depth explanation and suggested solution
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        self.spawn_collectors();
        let mut executor_handle = tokio::spawn(
            self.executor
                .take()
                .expect("executor should only be run once")
                .run_until_stopped(),
        );

        let wait_for_collectors = self.wait_for_collectors();
        let wait_for_executor = self.wait_for_executor();
        match tokio::try_join!(wait_for_collectors, wait_for_executor) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start searcher"),
        }

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = self.new_transactions_rx.recv() => self.bundle_pending_tx(rollup_tx),

                // submit signed sequencer txs to sequencer
                Some(join_result) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match join_result {
                        Ok(bundle) => self.handle_bundle_execution(bundle).await,
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "conversion task failed while trying to convert pending rollup transaction to signed sequencer transaction",
                        ),
                    }
                }

                Some((rollup, collector_exit)) = self.collector_tasks.join_next() => {
                    self.reconnect_exited_collector(rollup, collector_exit);
                }

                ret = &mut executor_handle => {
                    match ret {
                        Ok(Ok(())) => {
                            error!("executor task exited unexpectedly");
                        }
                        Ok(Err(e)) => {
                            error!(
                                error.message = %e,
                                error.cause_chain = ?e,
                                "executor returned with error",
                            );
                        }
                        Err(e) => {
                            error!(
                                error.message = %e,
                                error.cause_chain = ?e,
                                "executor task panicked",
                            );
                        }
                    }
                    break;
                }
            );
        }

        Ok(())
    }

    #[instrument(skip_all, fields(rollup))]
    fn reconnect_exited_collector(
        &mut self,
        rollup: String,
        exit_result: Result<eyre::Result<()>, JoinError>,
    ) {
        reconnect_exited_collector(
            &mut self.collector_statuses,
            &mut self.collector_tasks,
            self.new_transactions_tx.clone(),
            &self.rollups,
            rollup,
            exit_result,
        );
    }

    /// Spawns all collector on the collector task set.
    fn spawn_collectors(&mut self) {
        for (chain_id, collector) in self.collectors.drain() {
            self.collector_tasks
                .spawn(chain_id, collector.run_until_stopped());
        }
    }

    /// Waits for all collectors to come online.
    async fn wait_for_collectors(&self) -> eyre::Result<()> {
        use futures::{
            future::FutureExt as _,
            stream::{
                FuturesUnordered,
                StreamExt as _,
            },
        };
        let mut statuses = self
            .collector_statuses
            .iter()
            .map(|(chain_id, status)| {
                let mut status = status.clone();
                async move {
                    match status.wait_for(collector::Status::is_connected).await {
                        // `wait_for` returns a reference to status; throw it
                        // away because this future cannot return a reference to
                        // a stack local object.
                        Ok(_) => Ok(()),
                        // if an collector fails while waiting for its status, this
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
                        "collector for chain ID {chain_id} failed while waiting for it to become \
                         ready"
                    )
                });
            }
        }
        self.status.send_modify(|status| {
            status.all_collectors_connected = true;
        });
        Ok(())
    }

    async fn wait_for_executor(&self) -> eyre::Result<()> {
        // wait to receive executor status
        let mut status = self.executor_status.clone();
        status
            .wait_for(executor::Status::is_connected)
            .await
            .wrap_err("executor failed while waiting for it to become ready")?;

        // update searcher status
        self.status
            .send_modify(|status| status.executor_connected = true);

        Ok(())
    }
}

fn reconnect_exited_collector(
    collector_statuses: &mut HashMap<String, watch::Receiver<collector::Status>>,
    collector_tasks: &mut JoinMap<String, eyre::Result<()>>,
    new_transactions_tx: Sender<collector::Transaction>,
    rollups: &HashMap<String, String>,
    rollup: String,
    exit_result: Result<eyre::Result<()>, JoinError>,
) {
    match exit_result {
        Ok(Ok(())) => warn!("collector exited unexpectedly; reconnecting"),
        Ok(Err(e)) => {
            let error: &(dyn std::error::Error + 'static) = e.as_ref();
            warn!(error, "collector exit with error; reconnecting");
        }
        Err(e) => {
            let error = &e as &(dyn std::error::Error + 'static);
            warn!(error, "collector task failed; reconnecting");
        }
    }
    let Some(url) = rollups.get(&rollup) else {
        error!(
            "rollup should have had an entry in the rollup->url map but doesn't; not reconnecting \
             it"
        );
        return;
    };
    let collector = Collector::new(rollup.clone(), url.clone(), new_transactions_tx);
    collector_statuses.insert(rollup.clone(), collector.subscribe());
    collector_tasks.spawn(rollup, collector.run_until_stopped());
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use astria_core::sequencer::v1alpha1::RollupId;
    use ethers::types::Transaction;
    use tokio_util::task::JoinMap;

    use crate::searcher::collector::{
        self,
        Collector,
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
        let collector = Collector::new(rollup_name.clone(), rollup_url.clone(), tx.clone());
        let mut status = collector.subscribe();
        collector_tasks.spawn(rollup_name.clone(), collector.run_until_stopped());
        status
            .wait_for(collector::Status::is_connected)
            .await
            .unwrap();
        let expected_transaction = Transaction::default();
        let _ = mock_geth.push_tx(expected_transaction.clone()).unwrap();
        let collector_tx = rx.recv().await.unwrap();

        assert_eq!(
            RollupId::from_unhashed_bytes(&rollup_name),
            collector_tx.rollup_id,
        );
        assert_eq!(expected_transaction, collector_tx.inner);

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
            .wait_for(collector::Status::is_connected)
            .await
            .unwrap();
        let _ = mock_geth.push_tx(expected_transaction.clone()).unwrap();
        let collector_tx = rx.recv().await.unwrap();

        assert_eq!(
            RollupId::from_unhashed_bytes(&rollup_name),
            collector_tx.rollup_id,
        );
        assert_eq!(expected_transaction, collector_tx.inner);
    }
}
