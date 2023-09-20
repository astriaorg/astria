/// ! The `Executor` is responsible for:
/// - Nonce management
/// - Transaction signing
/// - Managing the connection to the sequencer
/// - Submitting transactions to the sequencer
use std::time::Duration;

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
    tx_sync,
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

/// The `Executor` interfaces with the sequencer. It handles account nonces, transaction signing,
/// and transaction submission.
/// The `Executor` receives `Vec<Action>` from the bundling logic, packages them with a nonce into
/// an `Unsigned`, then signs them with the sequencer key and submits to the sequencer.
/// Its `status` field indicates that connection to the sequencer node has been established.
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
    nonce: Option<u32>,
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
            nonce: None,
            address: sequencer_address,
        })
    }

    /// Return a reader to the status reporting channel
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Gets the next nonce to sign over if it exists and increments the stored nonce counter
    fn get_and_increment_nonce(&mut self) -> Option<u32> {
        self.nonce.map(|curr_nonce| {
            self.nonce = Some(curr_nonce + 1);
            curr_nonce
        })
    }

    /// Sugmits a signed transaction to the sequencer node.
    /// TODO: handle failed tx submission due to nonce
    async fn submit_tx(&self, signed_tx: SignedTransaction) -> eyre::Result<()> {
        let rsp = self
            .sequencer_client
            .submit_transaction_sync(signed_tx)
            .await
            .wrap_err("failed submitting transaction to sequencer")?;
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

    /// Signs and submits the bundle of actions to the sequencer.
    async fn sign_and_submit(&mut self, actions: Vec<Action>) -> eyre::Result<()> {
        let nonce = self
            .get_and_increment_nonce()
            .ok_or(eyre!("no nonce stored; cannot process bundle"))?;

        let tx = UnsignedTransaction {
            nonce,
            actions,
        }
        .into_signed(&self.sequencer_key);

        self.submit_tx(tx)
            .await
            .wrap_err("failed submitting signed actions to sequencer")?;
        Ok(())
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

        while let Some(bundle) = self.executor_rx.recv().await {
            if let Err(e) = self.sign_and_submit(bundle).await {
                // FIXME: currently this will fail both when there is an issue with the nonce and
                // when unable to reach the sequencer. As there is currently no error returned by
                // the sequencer for invalid nonces, there is nothing to handle. This should be
                // changed after #364 is merged in a followup PR to handle nonce failues.
                error!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    ?bundle,
                    "processing bundle failed",
                );
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

        // update stored nonce
        self.nonce = Some(nonce_response.nonce);
        info!(
            nonce_response.nonce,
            "retrieved initial nonce from sequencer successfully"
        );

        // update status to connected
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

    /// Wrapper around [`Client::submit_transaction_sync`] with a 1s timeout.
    async fn submit_transaction_sync(
        &self,
        signed_tx: SignedTransaction,
    ) -> eyre::Result<tx_sync::Response> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.inner.submit_transaction_sync(signed_tx),
        )
        .await
        .wrap_err("request timed out")?
        .wrap_err("RPC returned with error")
    }
}
