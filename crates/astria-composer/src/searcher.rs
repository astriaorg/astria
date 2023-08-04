use std::time::Duration;

use astria_sequencer::{
    accounts::types::Nonce,
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction,
        Signed as SignedSequencerTx,
        Unsigned as UnsignedSequencerTx,
    },
};
use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use ethers::{
    providers::{
        Provider,
        ProviderError,
        Ws,
    },
    types::Transaction,
};
use humantime::format_duration;
use tendermint::abci;
use tokio::{
    select,
    sync::watch,
    task::JoinSet,
};
use tracing::{
    debug,
    info,
    instrument,
    warn,
};

use crate::Config;

pub(super) struct Searcher {
    // The client for getting new pending transactions from the ethereum JSON RPC.
    eth_client: EthClient,
    // The client for submitting swrapped pending eth transactions to the astria sequencer.
    sequencer_client: SequencerClient,
    rollup_chain_id: String,
    status: watch::Sender<Status>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<eyre::Result<SignedSequencerTx>>,
    // Set of in-flight RPCs submitting signed transactions to the sequencer.
    // submission_tasks: JoinSet<eyre::Result<tx_sync::Response>>,
    submission_tasks: JoinSet<eyre::Result<()>>,
}

#[derive(Debug, Default)]
pub(crate) struct Status {
    geth_connected: bool,
    sequencer_connected: bool,
}

impl Status {
    pub(crate) fn is_ready(&self) -> bool {
        self.geth_connected && self.sequencer_connected
    }
}

/// A thin wrapper around [`Provider<Ws>`] to add timeouts.
///
/// Currently only provides a timeout around for `get_net_version`.
/// TODO: Also add timeouts for `subscribe_full_pendings_txs` (more
///       complex because it's a stream).
#[derive(Clone)]
struct EthClient {
    inner: Provider<Ws>,
}

impl EthClient {
    async fn connect(url: &str) -> Result<Self, ProviderError> {
        let inner = Provider::connect(url).await?;
        Ok(Self {
            inner,
        })
    }

    /// Wrapper around [`Provider::get_net_version`] with a 1s timeout.
    async fn get_net_version(&self) -> eyre::Result<String> {
        use ethers::providers::Middleware as _;
        tokio::time::timeout(Duration::from_secs(1), self.inner.get_net_version())
            .await
            .wrap_err("request timed out")?
            .wrap_err("RPC returned with error")
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

    /// Wrapper around [`Provider::get_net_version`] with a 1s timeout.
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
    pub(super) async fn from_config(cfg: &Config) -> eyre::Result<Self> {
        // connect to eth node
        let eth_client = EthClient::connect(&cfg.execution_url)
            .await
            .wrap_err("failed connecting to geth")?;

        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let rollup_chain_id = cfg.chain_id.clone();
        let (status, _) = watch::channel(Status::default());

        Ok(Searcher {
            eth_client,
            sequencer_client,
            rollup_chain_id,
            status,
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    async fn handle_pending_tx(&mut self, rollup_tx: Transaction) {
        let chain_id = self.rollup_chain_id.clone();

        self.conversion_tasks.spawn_blocking(move || {
            // FIXME: Needs to be altered when nonces are implemented in the sequencer
            // For now, each transaction is transmitted from a new account with nonce 0
            let sequencer_key = ed25519_consensus::SigningKey::new(rand::thread_rng());
            let nonce = Nonce::from(0);

            // Pack into sequencer tx
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));
            let unsigned_tx = UnsignedSequencerTx::new_with_actions(nonce, vec![seq_action]);

            // Sign transaction
            Ok(unsigned_tx.into_signed(&sequencer_key))
        });
    }

    fn handle_signed_tx(&mut self, tx: SignedSequencerTx) {
        use sequencer_client::SequencerClientExt as _;
        let client = self.sequencer_client.inner.clone();
        self.submission_tasks.spawn(async move {
            let rsp = client
                .submit_transaction_sync(tx.clone())
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
        use ethers::providers::{
            Middleware as _,
            StreamExt as _,
        };
        // FIXME: is waiting for geth even necessary if we were able to establish a websocket
        //        connection? Is there a scenario where we were able to establish a websocket
        //        connection, but where we are not able to make RPCs?
        let wait_for_eth = self.wait_for_eth(5, Duration::from_secs(5), 2.0);
        let wait_for_seq = self.wait_for_sequencer(5, Duration::from_secs(5), 2.0);
        match tokio::try_join!(wait_for_eth, wait_for_seq) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start searcher"),
        }
        let eth_client = self.eth_client.inner.clone();
        let mut tx_stream = eth_client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscriber eth client to full pending transactions")?;

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = tx_stream.next() => self.handle_pending_tx(rollup_tx).await,

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
            )
        }

        // FIXME: ensure that we can get here
        #[allow(unreachable_code)]
        Ok(())
    }

    /// Wait until a connection to geth is established.
    ///
    /// This function tries to establish a connection to geth by
    /// querying its net_version RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    /// An error is returned if calling the data availabilty failed for a total
    /// of `n_retries + 1` times.
    #[instrument(skip_all, fields(
        retries.max_number = n_retries,
        retries.initial_delay = %format_duration(delay),
        retries.exponential_factor = factor,
    ))]
    async fn wait_for_eth(
        &self,
        n_retries: usize,
        delay: Duration,
        factor: f32,
    ) -> eyre::Result<()> {
        use backon::{
            ExponentialBuilder,
            Retryable as _,
        };
        debug!("attempting to connect to geth");
        let backoff = ExponentialBuilder::default()
            .with_min_delay(delay)
            .with_factor(factor)
            .with_max_times(n_retries);
        let version = (|| {
            let client = self.eth_client.clone();
            // This is using `get_net_version` because that's what ethers' Middleware is
            // implementing. Maybe the `net_listening` RPC would be better, but ethers
            // does not have that.
            async move { client.get_net_version().await }
        })
        .retry(&backoff)
        .notify(|err, dur| warn!(error.msg = %err, retry_in = %format_duration(dur), "failed issuing RPC; retrying"))
        .await
        .wrap_err(
            "failed to retrieve latest height from data availability layer after several retries",
        )?;
        info!(version, rpc = "net_version", "RPC was successful");
        self.status.send_modify(|status| {
            status.geth_connected = true;
        });
        Ok(())
    }

    /// Wait until a connection to the sequencer is established.
    ///
    /// This function tries to establish a connection to the sequencer by
    /// querying its abci_info RPC. If it fails, it retries for another `n_retries`
    /// times with exponential backoff.
    ///
    /// # Errors
    /// An error is returned if calling the data availabilty failed for a total
    /// of `n_retries + 1` times.
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
        debug!("attempting to connect to data availability layer",);
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
            "failed to retrieve latest height from data availability layer after several retries",
        )?;
        self.status.send_modify(|status| {
            status.sequencer_connected = true;
        });
        Ok(())
    }
}
