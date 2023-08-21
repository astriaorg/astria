use std::{
    collections::HashMap,
    sync::{
        atomic::{
            AtomicU32,
            Ordering,
        },
        Arc,
    },
    time::Duration,
};

use astria_sequencer::{
    sequence,
    transaction::{
        self,
        Action,
    },
};
use color_eyre::eyre::{
    self,
    bail,
    eyre,
    WrapErr as _,
};
use ed25519_consensus::SigningKey;
use humantime::format_duration;
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
    debug,
    instrument,
    warn,
};

use self::executor::Executor;
use crate::Config;

mod collector;
mod executor;
mod rollup;

use collector::Collector;

/// the astria seqeuencer.
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
    // A channel on which to send the `Executor` bundles for attaching a nonce to, sign and submit
    bundles_tx: mpsc::Sender<Vec<Action>>,
    // Executor responsible for submitting transaction to the sequencer node.
    executor: Executor,
    executor_status: watch::Receiver<executor::Status>,
    // Set of in-flight RPCs submitting signed transactions to the sequencer.
    submission_tasks: JoinSet<eyre::Result<()>>,
}

#[derive(Debug, Default)]
pub(crate) struct Status {
    all_collectors_connected: bool,
    sequencer_connected: bool,
}

impl Status {
    pub(crate) fn is_ready(&self) -> bool {
        self.all_collectors_connected && self.sequencer_connected
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

        let (bundles_tx, bundles_rx) = mpsc::channel(256);

        let executor = Executor::new(&cfg.sequencer_url, &cfg.private_key, bundles_rx).await?;
        let executor_status = executor.subscribe();

        Ok(Searcher {
            status,
            collectors,
            collector_statuses,
            new_transactions,
            collector_tasks: tokio_util::task::JoinMap::new(),
            conversion_tasks: JoinSet::new(),
            executor,
            executor_status,
            bundles_tx,
            submission_tasks: JoinSet::new(),
        })
    }

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
            let seq_action = Action::SequenceAction(sequence::Action::new(chain_id, data));

            vec![seq_action]
        });
    }

    /// Runs the Searcher
    pub(super) async fn run(self) -> eyre::Result<()> {
        self.spawn_collectors();
        let _executor_task = tokio::spawn(self.executor.run_until_stopped());
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
                        Ok(actions) => self.bundles_tx.send(actions).await?,
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
        let mut status = self.executor_status.clone();
        match status.wait_for(executor::Status::is_connected).await {
            // `wait_for` returns a reference to status; throw it
            // away because this future cannot return a reference to
            // a stack local object.
            Ok(_) => Ok(()),
            // if an collector fails while waiting for its status, this
            // will return an error
            Err(e) => Err(e),
        }
        .wrap_err("failed waiting for executor to become ready")
    }
}
