use std::{
    sync::{
        atomic::{
            AtomicU32,
            Ordering,
        },
        Arc,
    },
    time::Duration,
};

use color_eyre::eyre::{
    self,
    bail,
    eyre,
    Context,
};
use ed25519_consensus::SigningKey;
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
    Address,
    NonceResponse,
    SequencerClientExt,
};
use tokio::sync::{
    mpsc,
    watch,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::Config;

pub(super) type StatusReceiver = watch::Receiver<Status>;
pub(super) type Sender = mpsc::Sender<Vec<Action>>;

pub(super) fn spawn(cfg: &Config) -> eyre::Result<(Sender, StatusReceiver)> {
    info!("Spawning Executor subtask for Searcher");
    // create channel for sending bundles to executor
    let (executor_tx, executor_rx) = mpsc::channel(256);
    let executor = Executor::new(&cfg.sequencer_url, &cfg.private_key, executor_rx)?;

    // create channel for receiving executor status
    let status_rx = executor.subscribe();

    // spawn executor task
    let join_handle = tokio::spawn(executor.run_until_stopped());

    // handle executor failure by logging
    tokio::task::spawn(async move {
        match join_handle.await {
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
    });

    Ok((executor_tx, status_rx))
}

#[derive(Debug)]
pub(super) struct Executor {
    // The status of this executor
    status: watch::Sender<Status>,
    // Channel for receving bundles to pack, sign, and submit
    executor_rx: mpsc::Receiver<Vec<Action>>,
    // The client for submitting wrapped and signed pending eth transactions to the astria
    // sequencer.
    sequencer_client: SequencerClient,
    // Private key used to sign sequencer transactions
    sequencer_key: SigningKey,
    // Nonce of the sequencer account we sign with
    nonce: Arc<AtomicU32>,
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

/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
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
            nonce: Arc::new(AtomicU32::new(0)),
            address: sequencer_address,
        })
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Return the current nonce
    fn nonce(&self) -> u32 {
        self.nonce.load(Ordering::Relaxed)
    }

    /// Gets the next nonce to sign over
    fn get_next_nonce(&mut self) -> u32 {
        // get current nonce and calculate next one
        let curr_nonce = self.nonce();
        let next_nonce = curr_nonce + 1;
        // save next nonce
        self.nonce.store(next_nonce, Ordering::Relaxed);
        curr_nonce
    }

    /// Creates an `Unsigned` from `Vec<Action>` using the current nonce.
    /// If the current nonce is not stored, fetches the latest nonce from the sequencer node.
    fn make_unsigned_tx(&mut self, actions: Vec<Action>) -> UnsignedTransaction {
        // get current nonce and increment nonce
        let curr_nonce = self.get_next_nonce();
        UnsignedTransaction {
            nonce: curr_nonce,
            actions,
        }
    }

    /// TODO
    async fn submit_tx(&self, signed_tx: SignedTransaction) -> eyre::Result<()> {
        let rsp = self
            .sequencer_client
            .inner
            .submit_transaction_sync(signed_tx)
            .await
            .wrap_err("failed to submit transaction to sequencer")?;
        if rsp.code.is_err() {
            bail!(
                "submitting transaction to sequencer returned with error code; code: `{code}`; \
                 log: `{log}`; hash: `{hash}`",
                code = rsp.code.value(),
                log = rsp.log,
                hash = rsp.hash,
            );
        }
        Ok(())
    }

    /// Run the Executor loop.
    /// Vec<Action> -> Unsigned -> Signed -> Submit
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        // set up connection to sequencer
        self.wait_for_sequencer(5, Duration::from_secs(5), 2.0)
            .await
            .wrap_err("failed connecting to sequencer")?;

        // for each bundle received, create unsigned tx, sign tx, then submit tx
        // Vec<Action> -> Unsigned -> Signed -> Submit
        while let Some(bundle) = self.executor_rx.recv().await {
            // create unsigned tx
            let unsigned_tx = self.make_unsigned_tx(bundle);
            // sign tx
            let signed_tx = unsigned_tx.into_signed(&self.sequencer_key);
            // submit tx
            self.submit_tx(signed_tx).await?;
            // FIXME: handle failed tx submission due to nonce
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

        let nonce_response = (|| {
            let client = self.sequencer_client.clone();
            async move {
                client.get_latest_nonce(self.address).await
            }
        })
        .retry(&backoff)
        .notify(|err, dur|
            warn!(error.msg = %err, retry_in = %format_duration(dur), "failed getting nonce for {:?}; retrying", self.address))
        .await
        .wrap_err(
            "failed to retrieve initial nonce from sequencer after several retries",
        )?;

        info!(
            nonce_response.nonce,
            "retrieved initial nonce from sequencer successfully"
        );
        self.nonce.store(nonce_response.nonce, Ordering::Relaxed);

        self.status.send_modify(|status| {
            status.is_connected = true;
        });
        Ok(())
    }
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
    async fn get_latest_nonce(&self, address: Address) -> eyre::Result<NonceResponse> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.inner.get_latest_nonce(address.0),
        )
        .await
        .wrap_err("request timed out")?
        .wrap_err("RPC returned with error")
    }
}
