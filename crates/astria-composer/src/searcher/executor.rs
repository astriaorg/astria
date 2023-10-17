/// ! The `Executor` is responsible for:
/// - Nonce management
/// - Transaction signing
/// - Managing the connection to the sequencer
/// - Submitting transactions to the sequencer
use std::{
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::{
    self,
    eyre,
    Context,
};
use ed25519_consensus::SigningKey;
use futures::{
    future::{
        Fuse,
        FusedFuture as _,
        FutureExt as _,
    },
    Future,
};
use humantime::format_duration;
use proto::native::sequencer::v1alpha1::{
    Action,
    SignedTransaction,
    UnsignedTransaction,
};
use secrecy::{
    ExposeSecret as _,
    SecretString,
    Zeroize as _,
};
use sequencer_client::{
    tendermint::endpoint::broadcast::tx_sync,
    Address,
    NonceResponse,
    SequencerClientExt,
};
use sequencer_types::abci_code::AbciCode;
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
#[derive(Debug)]
pub(super) struct Executor {
    // The status of this executor
    status: watch::Sender<Status>,
    // Channel receiver for bundles to pack, sign, and submit
    executor_rx: mpsc::Receiver<Vec<Action>>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: SequencerClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // Nonce of the sequencer account we sign with. Arc is for static futures. Should only be held
    // by task at a time.
    nonce: Arc<Option<u32>>,
    // The sequencer address associated with the private key
    address: Address,
}

#[derive(Debug)]
pub(super) struct Status {
    is_connected: bool,
}

impl Status {
    pub(super) fn new() -> Self {
        Self {
            is_connected: false,
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        self.is_connected
    }
}

type ExecutionFut =
    Fuse<Pin<Box<dyn Future<Output = Result<SignedTransaction, ExecutionError>> + Send>>>;

impl Executor {
    pub(super) fn new(
        sequencer_url: &str,
        private_key: &SecretString,
        executor_rx: mpsc::Receiver<Vec<Action>>,
    ) -> eyre::Result<Self> {
        // connect to sequencer node
        let sequencer_client =
            SequencerClient::new(sequencer_url).wrap_err("failed constructing sequencer client")?;

        // create signing key for sequencer txs
        let mut private_key_bytes: [u8; 32] = hex::decode(private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key =
            SigningKey::try_from(private_key_bytes).wrap_err("failed to parse sequencer key")?;
        private_key_bytes.zeroize();

        // create address from signing key
        let sequencer_address = Address::from_verification_key(sequencer_key.verification_key());

        // create channel for status reporting
        let (status, _) = watch::channel(Status::new());

        Ok(Self {
            status,
            executor_rx,
            sequencer_client,
            sequencer_key,
            nonce: Arc::new(None),
            address: sequencer_address,
        })
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Returns the transaction execution fused future. The future will populate the nonce, sign and
    /// submit the bundle of actions to the sequencer.
    fn bundle_execution(&mut self, bundle: Vec<Action>) -> ExecutionFut {
        let curr_nonce =
            Arc::get_mut(&mut self.nonce).expect("should only be called once at a time");
        let sequencer_client = self.sequencer_client.clone();
        let address = self.address;
        let sequencer_key = &self.sequencer_key;

        async move {
            let nonce = get_and_increment_nonce(curr_nonce, sequencer_client.clone(), address)
                .await
                .map_err(ExecutionError::NonceRetreivalFailed)?;

            let tx = UnsignedTransaction {
                nonce,
                actions: bundle,
            }
            .into_signed(&sequencer_key);

            let submission_rsp = sequencer_client
                .submit_transaction_sync(tx.clone())
                .await
                .map_err(|e| ExecutionError::TransactionSubmissionFailed {
                    error: e,
                    transaction: tx.clone(),
                })?;

            match AbciCode::from_tendermint(submission_rsp.code) {
                Some(AbciCode::OK) => Ok(tx),
                Some(AbciCode::INVALID_NONCE) => Err(ExecutionError::InvalidNonce {
                    nonce,
                    transaction: tx,
                }),
                _ => Err(ExecutionError::UnknownDeliverTxFailure {
                    code: submission_rsp.code,
                    transaction: tx,
                }),
            }
        }
        .boxed()
        .fuse()
    }

    fn bundle_resubmission_from_transaction(
        &mut self,
        transaction: SignedTransaction,
    ) -> ExecutionFut {
        // reset nonce
        info!(old_nonce = *self.nonce, "resetting nonce to None");
        *Arc::get_mut(&mut self.nonce)
            .expect("should only be called once at a time, this is a bug") = None;

        // get the bundle out from the signed tx
        let bundle = {
            let (_, _, unsigned) = transaction.into_parts();
            unsigned.actions
        };

        // reexecute bundle to attach new nonce
        self.bundle_execution(bundle)
    }

    /// Run the Executor loop, calling `process_bundle` on each bundle received from the channel.
    ///
    /// # Errors
    /// An error is returned if connecting to the sequencer fails.
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        // set up connection to sequencer
        self.init_nonce_from_sequencer(5, Duration::from_secs(5), 2.0)
            .await
            .wrap_err("failed retrieving initial nonce from sequencer")?;

        let mut execution_fut = Fuse::terminated();
        loop {
            select! {
                // in flight submission
                Some(bundle) = self.executor_rx.recv(), if execution_fut.is_terminated() => {
                    debug!(bundle = ?bundle, "received bundle from channel");
                    execution_fut = self.bundle_execution(bundle);
                },
                // resubmission
                ret = &mut execution_fut, if !execution_fut.is_terminated() => {
                    match ret {
                       Err(ExecutionError::InvalidNonce {
                               nonce,
                               transaction,
                           })
                           => {
                               warn!(
                                   nonce,
                                   "invalid nonce returned from sequencer; retrieving new nonce and \
                                    resubmitting the transaction"
                               );
                               execution_fut = self.bundle_resubmission_from_transaction(transaction)
                           }
                           Err(ExecutionError::UnknownDeliverTxFailure {
                               code,
                               transaction,
                           }) => {
                               warn!(
                                   code=?code,
                                   transaction = ?transaction,
                                   "unknown error code returned from sequencer; skipping this \
                                    transaction"
                               );
                           }
                           Err(ExecutionError::NonceRetreivalFailed(e)) => {
                               error!(error.msg = %e, "failed to retrieve nonce from sequencer; executor shutting down");
                               break;
                           }
                           Err(ExecutionError::TransactionSubmissionFailed{ error:e, transaction }) => {
                               error!(error.msg = %e, transaction = ?transaction, "failed to submit transaction to sequencer; executor shutting down");
                               break;
                           }
                       Ok(tx) => {
                           let nonce = tx.unsigned_transaction().nonce;
                           info!(
                               tx = ?tx,
                               nonce = ?nonce,
                               "transaction submitted to sequencer successfully with nonce {}",
                               nonce
                           );
                       }
                    }
                }
            }
        }
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
    async fn init_nonce_from_sequencer(
        &mut self,
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
        let nonce_response = (|| {
            let client = self.sequencer_client.clone();
            let address = self.address;
            async move {
                client.get_latest_nonce(address).await
            }
        })
        .retry(&backoff)
        .notify(|err, dur|
            warn!(error.msg = %err, retry_in = %format_duration(dur), "failed getting nonce for {:?}; retrying", self.address))
        .await
        .wrap_err(
            "failed to retrieve initial nonce from sequencer after several retries",
        )?;

        self.nonce = Arc::new(Some(nonce_response.nonce));
        info!(
            nonce_response.nonce,
            "retrieved initial nonce from sequencer successfully"
        );

        self.status.send_modify(|status| {
            status.is_connected = true;
        });
        Ok(())
    }
}

/// Gets the next nonce to sign over and increments the given counter.
/// If the current counter is `None`, this will fetch the latest nonce from the sequencer.
async fn get_and_increment_nonce(
    nonce: &mut Option<u32>,
    sequencer_client: SequencerClient,
    address: Address,
) -> Result<u32, SequencerClientError> {
    if let Some(nonce) = nonce {
        let curr_nonce = *nonce;
        *nonce = *nonce + 1;

        info!(prev_nonce = *nonce, curr_nonce, "incremented nonce");

        Ok(curr_nonce)
    } else {
        debug!("nonce currently set to None. retrieving new nonce from sequencer");
        let rsp = sequencer_client.get_latest_nonce(address).await?;
        *nonce = Some(rsp.nonce + 1);

        info!(nonce = *nonce, "retrieved nonce from sequencer");

        Ok(rsp.nonce)
    }
}

#[derive(Debug, thiserror::Error)]
enum ExecutionError {
    #[error("failed to communicate with sequencer")]
    NonceRetreivalFailed(SequencerClientError),
    #[error("failed to submit sequencer transaction")]
    TransactionSubmissionFailed {
        error: SequencerClientError,
        transaction: SignedTransaction,
    },
    #[error("transaction submission failed due to invalid nonce")]
    InvalidNonce {
        nonce: u32,
        transaction: SignedTransaction,
    },
    #[error("transaction submission failed with unknown error code")]
    UnknownDeliverTxFailure {
        code: tendermint::abci::Code,
        transaction: SignedTransaction,
    },
}

#[derive(Debug, thiserror::Error)]
enum SequencerClientError {
    #[error("request timed out")]
    Timeout,
    #[error("RPC request failed")]
    RequestFailed(#[source] sequencer_client::extension_trait::Error),
}
/// A thin wrapper around [`sequencer_client::Client`] to add timeouts.
///
/// Currently only provides a timeout for `abci_info`.
#[derive(Clone, Debug)]
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

    /// Wrapper around [`Client::get_latest_nonce`] with a 1s timeout.
    async fn get_latest_nonce(
        &self,
        address: Address,
    ) -> Result<NonceResponse, SequencerClientError> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.inner.get_latest_nonce(address.0),
        )
        .await
        .map_err(|_e| SequencerClientError::Timeout)?
        .map_err(SequencerClientError::RequestFailed)
    }

    /// Wrapper around [`Client::submit_transaction_sync`] with a 1s timeout.
    async fn submit_transaction_sync(
        &self,
        signed_tx: SignedTransaction,
    ) -> Result<tx_sync::Response, SequencerClientError> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.inner.submit_transaction_sync(signed_tx),
        )
        .await
        .map_err(|_e| SequencerClientError::Timeout)?
        .map_err(SequencerClientError::RequestFailed)
    }
}
