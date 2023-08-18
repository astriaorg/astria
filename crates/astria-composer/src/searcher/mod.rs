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

use astria_sequencer::transaction;
use color_eyre::eyre::{
    self,
    bail,
    eyre,
    WrapErr as _,
};
use ed25519_consensus::SigningKey;
use humantime::format_duration;
use secrecy::{
    ExposeSecret as _,
    Zeroize as _,
};
use sequencer_client::{
    Address,
    SequencerClientExt as _,
};
use tendermint::abci;
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

use crate::Config;

mod collector;
mod rollup;

use collector::Executor;

/// the astria seqeuencer.
pub(super) struct Searcher {
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: SequencerClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // Nonce of the sequencer account we sign with
    sequencer_nonce: Arc<AtomicU32>,
    // Channel to report the internal status of the searcher to other parts of the system.
    status: watch::Sender<Status>,
    collectors: HashMap<String, Executor>,
    collector_statuses: HashMap<String, watch::Receiver<collector::Status>>,
    // A channel on which the searcher receives transactions from its executors.
    new_transactions: Receiver<collector::Transaction>,
    collector_tasks: tokio_util::task::JoinMap<String, eyre::Result<()>>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<eyre::Result<transaction::Signed>>,
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

/// A thin wrapper around [`sequencer_client::Client`] to add timeouts.
///
/// Currently only provides a timeout for `abci_info`.
#[derive(Clone)]
struct SequencerClient {
    inner: sequencer_client::HttpClient,
}

impl SequencerClient {
    #[instrument]
    fn new(url: &str) -> eyre::Result<Self> {
        let inner = sequencer_client::HttpClient::new(url)
            .wrap_err("failed to construct sequencer client")?;
        Ok(Self {
            inner,
        })
    }

    /// Wrapper around [`Client::abci_info`] with a 1s timeout.
    async fn abci_info(self) -> eyre::Result<abci::response::Info> {
        use sequencer_client::Client as _;
        tokio::time::timeout(Duration::from_secs(1), self.inner.abci_info())
            .await
            .wrap_err("request timed out")?
            .wrap_err("RPC returned with error")
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
                tokio::spawn(Executor::new(chain_id, url, tx_sender.clone()))
                    .map(move |result| (task_name, result))
            })
            .collect::<futures::stream::FuturesUnordered<_>>();
        // TODO(superfluffy): allow aborting this using `futures::stream::AbortHandle`
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

        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let (status, _) = watch::channel(Status::default());

        // create signing key for sequencer txs
        let mut private_key_bytes: [u8; 32] = hex::decode(cfg.private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key =
            SigningKey::try_from(private_key_bytes).wrap_err("failed to parse sequencer key")?;
        private_key_bytes.zeroize();

        Ok(Searcher {
            sequencer_client,
            sequencer_key,
            sequencer_nonce: Arc::new(AtomicU32::new(0)),
            status,
            collectors,
            collector_statuses,
            new_transactions,
            collector_tasks: tokio_util::task::JoinMap::new(),
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    fn handle_pending_tx(&mut self, tx: collector::Transaction) {
        use astria_sequencer::{
            accounts::types::Nonce,
            sequence,
            transaction::action,
        };

        let collector::Transaction {
            chain_id,
            inner: rollup_tx,
        } = tx;

        let sequencer_key = self.sequencer_key.clone();
        let nonce = self.sequencer_nonce.clone();

        self.conversion_tasks.spawn_blocking(move || {
            // Pack into sequencer tx
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = action::Action::SequenceAction(sequence::Action::new(chain_id, data));

            // get current nonce and increment nonce
            let curr_nonce = nonce.fetch_add(1, Ordering::Relaxed);
            let unsigned_tx =
                transaction::Unsigned::new_with_actions(Nonce::from(curr_nonce), vec![seq_action]);

            // Sign transaction
            Ok(unsigned_tx.into_signed(&sequencer_key))
        });
    }

    fn handle_signed_tx(&mut self, tx: transaction::Signed) {
        use sequencer_client::SequencerClientExt as _;
        let client = self.sequencer_client.inner.clone();
        self.submission_tasks.spawn(async move {
            let rsp = client
                .submit_transaction_sync(tx)
                .await
                .wrap_err("failed to submit transaction to sequencer")?;
            if rsp.code.is_err() {
                bail!(
                    "submitting transaction to sequencer returned with error code; code: \
                     `{code}`; log: `{log}`; hash: `{hash}`",
                    code = rsp.code.value(),
                    log = rsp.log,
                    hash = rsp.hash,
                );
            }
            Ok(())
        });
    }

    /// Runs the Searcher
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        self.spawn_executors();
        let wait_for_collectors = self.wait_for_collectors();
        let wait_for_seq = self.wait_for_sequencer(5, Duration::from_secs(5), 2.0);
        match tokio::try_join!(wait_for_collectors, wait_for_seq) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start searcher"),
        }
        // set initial sequencer nonce
        let address = Address::from_verification_key(self.sequencer_key.verification_key());
        let nonce_response = self
            .sequencer_client
            .inner
            .get_latest_nonce(address.0)
            .await
            .wrap_err("failed to query sequencer for nonce")?;
        self.sequencer_nonce
            .store(nonce_response.nonce, Ordering::Relaxed);

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = self.new_transactions.recv() => self.handle_pending_tx(rollup_tx),

                // submit signed sequencer txs to sequencer
                Some(join_result) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match join_result {
                        Ok(Ok(signed_tx)) => self.handle_signed_tx(signed_tx),
                        Ok(Err(e)) => warn!(error.message = %e, error.cause_chain = ?e, "failed to sign sequencer transaction"),
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
                            // TODO: Decide what to do if submitting to sequencer failed. Should it be resubmitted?
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

    /// Spawns all executors on the executor task set.
    fn spawn_executors(&mut self) {
        for (chain_id, executor) in self.collectors.drain() {
            self.collector_tasks
                .spawn(chain_id, executor.run_until_stopped());
        }
    }

    /// Waits for all executors to come online.
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
                        // if an executor fails while waiting for its status, this
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
                        "executor for chain ID {chain_id} failed while waiting for it to become \
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

    /// Wait until a connection to the sequencer is established.
    ///
    /// This function tries to establish a connection to the sequencer by
    /// querying its `abci_info` RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    ///
    /// An error is returned if calling the sequencer failed for `n_retries + 1` times.
    #[instrument(skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    async fn wait_for_sequencer(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> eyre::Result<()> {
        use backon::{
            ExponentialBuilder,
            Retryable as _,
        };
        debug!("attempting to connect to sequencer",);
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        (|| {
            let client = self.sequencer_client.clone();
            async move { client.abci_info().await }
        })
        .retry(&backoff)
        .notify(|err, dur| warn!(error.msg = %err, retry_in = %format_duration(dur), "failed getting abci info; retrying"))
        .await
        .wrap_err(
            "failed to retrieve abci info from sequencer after several retries",
        )?;
        self.status.send_modify(|status| {
            status.sequencer_connected = true;
        });
        Ok(())
    }
}
