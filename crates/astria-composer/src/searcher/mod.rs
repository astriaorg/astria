use std::collections::HashMap;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use proto::native::sequencer::v1alpha1::{
    Action,
    SequenceAction,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            Receiver,
        },
        watch,
    },
    task::JoinSet,
};
use tracing::{
    error,
    warn,
};

use crate::{
    searcher::executor::Executor,
    Config,
};

mod collector;
mod executor;
mod rollup;

use collector::Collector;

const BUNDLE_CHANNEL_SIZE: usize = 256;

/// A Searcher collates transactions from multiple rollups and bundles them into
/// Astria sequencer transactions that are then passed on to the
/// Shared Sequencer. The rollup transactions that make up these sequencer transactions
/// have have the property of atomic inclusion, i.e. if they are submitted to the
/// sequencer, all of them are going to be executed in the same Astria block.
pub(super) struct Searcher {
    // Channel to report the internal status of the searcher to other parts of the system.
    status: watch::Sender<Status>,
    collectors: HashMap<String, Collector>,
    collector_statuses: HashMap<String, watch::Receiver<collector::Status>>,
    // A channel on which the searcher receives transactions from its collectors.
    new_transactions: Receiver<collector::Transaction>,
    collector_tasks: tokio_util::task::JoinMap<String, eyre::Result<()>>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<Vec<Action>>,
    // The Executor object that is responsible for signing and submitting sequencer transactions.
    executor: Option<Executor>,
    // A channel on which to send the `Executor` bundles for attaching a nonce to, sign and submit
    executor_tx: mpsc::Sender<Vec<Action>>,
    // Channel from which to read the internal status of the executor.
    executor_status: watch::Receiver<executor::Status>,
    // Set of in-flight RPCs submitting signed transactions to the sequencer.
    submission_tasks: JoinSet<eyre::Result<()>>,
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
    pub(super) async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        use futures::{
            FutureExt as _,
            StreamExt as _,
        };
        use rollup::Rollup;
        let rollups = cfg
            .rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <chain_id>::<url> pairs as rollups")?;

        let (tx_sender, new_transactions) = mpsc::channel(256);

        let mut create_collectors = rollups
            .into_iter()
            .map(|(chain_id, url)| {
                let task_name = chain_id.clone();
                tokio::spawn(Collector::new(chain_id, url, tx_sender.clone()))
                    .map(move |result| (task_name, result))
            })
            .collect::<futures::stream::FuturesUnordered<_>>();
        // TODO(https://github.com/astriaorg/astria/issues/287): add timeouts or abort handles
        // so this doesn't stall the entire server from coming up.
        let mut collectors = HashMap::new();
        while let Some((chain_id, join_result)) = create_collectors.next().await {
            match join_result {
                Err(err) => {
                    return Err(err).wrap_err_with(|| {
                        format!("task starting collector for {chain_id} panicked")
                    });
                }
                Ok(Err(err)) => {
                    return Err(err)
                        .wrap_err_with(|| format!("failed starting collector for {chain_id}"));
                }
                Ok(Ok(collector)) => {
                    collectors.insert(chain_id, collector);
                }
            }
        }

        let collector_statuses = collectors
            .iter()
            .map(|(chain_id, collector)| (chain_id.clone(), collector.subscribe()))
            .collect();

        let (status, _) = watch::channel(Status::default());

        // create channel for sending bundles to executor
        let (executor_tx, executor_rx) = mpsc::channel(BUNDLE_CHANNEL_SIZE);
        let executor = Executor::new(&cfg.sequencer_url, &cfg.private_key, executor_rx)
            .context("executor construction from config failed")?;

        // create channel for receiving executor status
        let executor_status = executor.subscribe();

        Ok(Searcher {
            status,
            collectors,
            collector_statuses,
            new_transactions,
            collector_tasks: tokio_util::task::JoinMap::new(),
            conversion_tasks: JoinSet::new(),
            executor_tx,
            executor_status,
            executor: Some(executor),
            submission_tasks: JoinSet::new(),
        })
    }

    /// Other modules can use this to get notified of changes to the Searcher state
    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    fn bundle_pending_tx(&mut self, tx: collector::Transaction) {
        let collector::Transaction {
            chain_id,
            inner: rollup_tx,
        } = tx;

        // rollup transaction data serialization is a heavy compute task, so it is spawned
        // on tokio's blocking threadpool
        self.conversion_tasks.spawn_blocking(move || {
            // Pack into a vector of sequencer actions
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = Action::Sequence(SequenceAction {
                chain_id,
                data,
            });

            vec![seq_action]
        });
    }

    /// Starts the searcher and runs it until failure
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        self.spawn_collectors();
        let mut exectuor_handle = self.spawn_executor();
        let wait_for_collectors = self.wait_for_collectors();
        let wait_for_executor = self.wait_for_executor();
        match tokio::try_join!(wait_for_collectors, wait_for_executor) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start searcher"),
        }

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = self.new_transactions.recv() => self.bundle_pending_tx(rollup_tx),

                // submit signed sequencer txs to sequencer
                Some(join_result) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match join_result {
                        Ok(actions) => self.executor_tx.send(actions).await?,
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "conversion task failed while trying to convert pending eth transaction to signed sequencer transaction",
                        ),
                    }
                }

                // handle failed sequencer tx submissions
                Some(join_result) = self.submission_tasks.join_next(), if !self.submission_tasks.is_empty() => {
                    match join_result {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) =>
                            // TODO(https://github.com/astriaorg/astria/issues/246): What to do if
                            // submitting fails. Resubmit?
                            warn!(error.message = %e, error.cause_chain = ?e, "failed to submit signed sequencer transaction to sequencer"),
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "submission task failed while trying to submit signed sequencer transaction to sequencer",
                        ),
                    }
                }

                _ = &mut exectuor_handle => { todo!("handle executor failure"); }
            );
        }

        // FIXME: ensure that we can get here
        #[allow(unreachable_code)]
        Ok(())
    }

    /// Spawns all collector on the collector task set.
    fn spawn_collectors(&mut self) {
        for (chain_id, collector) in self.collectors.drain() {
            self.collector_tasks
                .spawn(chain_id, collector.run_until_stopped());
        }
    }

    fn spawn_executor(&mut self) -> tokio::task::JoinHandle<()> {
        // spawn executor task
        let handle = tokio::task::spawn(
            self.executor
                .take()
                .expect("executor should only be run once")
                .run_until_stopped(),
        );

        // return handle to task that logs why executor exits
        tokio::task::spawn(async move {
            match handle.await {
                Ok(Ok(())) => {
                    error!("executor task exited unexpectedly");
                }
                Ok(Err(e)) => {
                    error!(
                        error.message = %e,
                        error.cause_chain = ?e,
                        "executor task failed unexpectedly with error",
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
        })
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
        async move {
            match status.wait_for(executor::Status::is_connected).await {
                // `wait_for` returns a reference to status; throw it
                // away because this future cannot return a reference to
                // a stack local object.
                Ok(_) => Ok(()),
                // if an collector fails while waiting for its status, this
                // will return an error
                Err(e) => Err(e),
            }
        }
        .await
        .wrap_err("executor failed while waiting for it to become ready")?;

        // update searcher status
        self.status
            .send_modify(|status| status.executor_connected = true);

        Ok(())
    }
}
