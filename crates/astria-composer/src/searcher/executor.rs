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
    tendermint_rpc::endpoint::broadcast::tx_sync,
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

type NonceFut =
    Fuse<Pin<Box<dyn Future<Output = Result<UnsignedTransaction, SequencerClientError>> + Send>>>;
type SubmissionFut =
    Fuse<Pin<Box<dyn Future<Output = Result<SignedTransaction, ExecutionError>> + Send>>>;

impl Executor {
    pub(super) fn new(
        sequencer_url: &str,
        private_key: &SecretString,
        executor_rx: mpsc::Receiver<Vec<Action>>,
    ) -> eyre::Result<Self> {
        let sequencer_client =
            SequencerClient::new(sequencer_url).wrap_err("failed constructing sequencer client")?;

        let mut private_key_bytes: [u8; 32] = hex::decode(private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key =
            SigningKey::try_from(private_key_bytes).wrap_err("failed to parse sequencer key")?;
        private_key_bytes.zeroize();

        let sequencer_address = Address::from_verification_key(sequencer_key.verification_key());

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

    /// Returns a fused future that attaches the nonce to the given bundle. If the stored nonce is
    /// `None`, the future will retrieve the latest nonce from the sequencer.
    fn attach_nonce(&mut self, bundle: Vec<Action>) -> NonceFut {
        let sequencer_client = self.sequencer_client.clone();
        let address = self.address;
        let nonce = *self.nonce;

        async move {
            if let Some(curr_nonce) = nonce {
                debug!(nonce = curr_nonce, "attached nonce to unsigned transaction");

                Ok(UnsignedTransaction {
                    nonce: curr_nonce,
                    actions: bundle,
                })
            } else {
                debug!("nonce currently set to None. retrieving new nonce from sequencer");
                let rsp = sequencer_client.get_latest_nonce(address).await?;

                info!(nonce = rsp.nonce, "retrieved nonce from sequencer");

                Ok(UnsignedTransaction {
                    nonce: rsp.nonce,
                    actions: bundle,
                })
            }
        }
        .boxed()
        .fuse()
    }

    /// Returns the transaction execution fused future. The future will sign the
    /// `UnsignedTransaction` and submit the resulting `SignedTransaction` to the sequencer.
    fn sign_and_submit(&mut self, unsigned_tx: UnsignedTransaction) -> SubmissionFut {
        let sequencer_client = self.sequencer_client.clone();
        let sequencer_key = self.sequencer_key.clone();

        async move {
            let signed_tx = unsigned_tx.into_signed(&sequencer_key);

            debug!(?signed_tx, "submitting signed transaction to the sequencer");
            let submission_rsp = sequencer_client
                .submit_transaction_sync(signed_tx.clone())
                .await
                .map_err(|e| ExecutionError::TransactionSubmissionFailed {
                    source: e,
                    transaction: signed_tx.clone(),
                })?;

            match AbciCode::from_tendermint(submission_rsp.code) {
                Some(AbciCode::OK) => Ok(signed_tx),
                Some(AbciCode::INVALID_NONCE) => Err(ExecutionError::InvalidNonce(signed_tx)),
                _ => Err(ExecutionError::UnknownDeliverTxFailure {
                    code: submission_rsp.code,
                    transaction: signed_tx,
                }),
            }
        }
        .boxed()
        .fuse()
    }

    /// Resets the nonce to `None` and returns a nonce refetch fused future for the payload of the
    /// `SignedTransaction`.
    fn bundle_resubmission_from_transaction(&mut self, transaction: SignedTransaction) -> NonceFut {
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
        self.attach_nonce(bundle)
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

        let mut submission_fut = Fuse::terminated();
        let mut nonce_fut = Fuse::terminated();
        loop {
            select! {
                // receive new bundle for processing
                Some(bundle) = self.executor_rx.recv(), if nonce_fut.is_terminated() && submission_fut.is_terminated() => {
                    debug!(bundle = ?bundle, "executor received bundle for processing");
                    nonce_fut = self.attach_nonce(bundle);
                },
                // attach nonce
                ret = &mut nonce_fut, if !nonce_fut.is_terminated() => {
                    match ret {
                        Ok(unsigned_tx) => {
                            // update self.nonce for next transaction
                            *Arc::get_mut(&mut self.nonce).expect("should only be called once at a time") =
                                Some(unsigned_tx.nonce + 1);

                            submission_fut = self.sign_and_submit(unsigned_tx);
                        }
                        Err(e) => {
                            error!(error.msg = %e, "failed to retrieve nonce from sequencer; executor shutting down");
                            break;
                        }
                    }
                }
                // submit to sequencer
                ret = &mut submission_fut, if !submission_fut.is_terminated() => {
                    match ret {
                        Err(ExecutionError::InvalidNonce(transaction)) => {
                            let nonce = transaction.unsigned_transaction().nonce;
                            warn!(
                                nonce,
                                "invalid nonce error returned from sequencer; retrieving new nonce and \
                                 resubmitting the transaction"
                            );
                            nonce_fut = self.bundle_resubmission_from_transaction(transaction);
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
                        Err(ExecutionError::TransactionSubmissionFailed {
                            source,
                            transaction
                        }) => {
                            error!(
                                error.msg = %source,
                                transaction = ?transaction,
                                "failed to submit transaction to sequencer; executor shutting down");
                            break;
                        }
                        Ok(tx) => {
                            let nonce = tx.unsigned_transaction().nonce;
                            info!(
                                tx = ?tx,
                                nonce = ?nonce,
                                "transaction submitted to sequencer successfully with nonce"
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
            async move { client.get_latest_nonce(address).await }
        })
        .retry(&backoff)
        .notify(|err, dur| {
            warn!(
                error.message = %err,
                error.cause = ?err,
                retry_in = %format_duration(dur),
                address = %self.address,
                "failed getting nonce; retrying",
            );
        })
        .await
        .wrap_err("failed to retrieve initial nonce from sequencer after several retries")?;

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

#[derive(Debug, thiserror::Error)]
enum ExecutionError {
    #[error("failed to submit sequencer transaction")]
    TransactionSubmissionFailed {
        #[source]
        source: SequencerClientError,
        transaction: SignedTransaction,
    },
    #[error("transaction submission failed due to invalid nonce")]
    InvalidNonce(SignedTransaction),
    #[error("transaction submission failed with unknown error code")]
    UnknownDeliverTxFailure {
        code: tendermint::abci::Code,
        transaction: SignedTransaction,
    },
}

#[derive(Debug, thiserror::Error)]
enum SequencerClientError {
    #[error("request timed out")]
    Timeout(#[source] tokio::time::error::Elapsed),
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
        .map_err(SequencerClientError::Timeout)?
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
        .map_err(SequencerClientError::Timeout)?
        .map_err(SequencerClientError::RequestFailed)
    }
}
