use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
    protocol::transaction::v1::{
        Action,
        TransactionBody,
    },
};
use astria_sequencer_client::{
    tendermint_rpc::endpoint::tx::Response,
    Client as _,
    HttpClient,
    SequencerClientExt as _,
};
use color_eyre::eyre::{
    self,
    ensure,
    eyre,
    WrapErr as _,
};
use tokio::{
    sync::Mutex,
    time::{
        self,
        Duration,
        Instant,
    },
};
use tracing::{
    debug,
    debug_span,
    instrument,
    warn,
    Instrument as _,
    Span,
};

pub(crate) async fn submit_transaction(
    sequencer_url: &str,
    chain_id: String,
    prefix: &str,
    private_key: &str,
    action: Action,
) -> eyre::Result<Response> {
    let sequencer_client =
        HttpClient::new(sequencer_url).wrap_err("failed constructing http sequencer client")?;

    let sequencer_key = signing_key_from_private_key(private_key)?;

    let from_address = address_from_signing_key(&sequencer_key, prefix)?;
    println!("sending tx from address: {from_address}");

    let nonce_res = sequencer_client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    let tx = TransactionBody::builder()
        .nonce(nonce_res.nonce)
        .chain_id(chain_id)
        .actions(vec![action])
        .try_build()
        .wrap_err("failed to construct a transaction")?
        .sign(&sequencer_key);
    let res = sequencer_client
        .submit_transaction_sync(tx)
        .await
        .wrap_err("failed to submit transaction")?;

    ensure!(res.code.is_ok(), "failed to check tx: {}", res.log);

    let tx_response = wait_for_tx_inclusion(sequencer_client, res.hash)
        .await
        .wrap_err("failed waiting for tx inclusion")?;

    ensure!(
        tx_response.tx_result.code.is_ok(),
        "failed to execute tx: {}",
        tx_response.tx_result.log
    );
    Ok(tx_response)
}

pub(crate) fn signing_key_from_private_key(private_key: &str) -> eyre::Result<SigningKey> {
    // Decode the hex string to get the private key bytes
    let private_key_bytes: [u8; 32] = hex::decode(private_key)
        .wrap_err("failed to decode private key bytes from hex string")?
        .try_into()
        .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;

    // Create and return a signing key from the private key bytes
    Ok(SigningKey::from(private_key_bytes))
}

pub(crate) fn address_from_signing_key(
    signing_key: &SigningKey,
    prefix: &str,
) -> eyre::Result<Address> {
    // Build the address using the public key from the signing key
    let from_address = Address::builder()
        .array(*signing_key.verification_key().address_bytes())
        .prefix(prefix)
        .try_build()
        .wrap_err("failed constructing a valid from address from the provided prefix")?;

    // Return the generated address
    Ok(from_address)
}

#[instrument(fields(%tx_hash), skip_all, ret)]
pub(crate) async fn wait_for_tx_inclusion(
    client: HttpClient,
    tx_hash: tendermint::hash::Hash,
) -> eyre::Result<Response> {
    // The min duration to sleep after receiving a GetTx response and sending the next request.
    const MIN_POLL_INTERVAL: Duration = Duration::from_millis(100);
    // The max duration to sleep after receiving a GetTx response and sending the next request.
    const MAX_POLL_INTERVAL: Duration = Duration::from_millis(2000);
    // How long to wait before starting to log at the warn level.
    const START_WARNING_DELAY: Duration = Duration::from_millis(2000);
    // The minimum duration between logging errors.
    const LOG_ERROR_INTERVAL: Duration = Duration::from_millis(2000);

    let start = Instant::now();
    let logged_at = Arc::new(Mutex::new(start));

    let log_if_due = |logged_at: Arc<Mutex<Instant>>, error: String| async move {
        let mut logged_at = logged_at.lock().await;
        if logged_at.elapsed() <= LOG_ERROR_INTERVAL {
            return;
        }
        *logged_at = Instant::now();
        if start.elapsed() < START_WARNING_DELAY {
            debug! {
                error,
                %tx_hash,
                elapsed_seconds = start.elapsed().as_secs_f32(),
                "waiting to confirm transaction inclusion"
            }
        } else {
            warn!(
                error,
                %tx_hash,
                elapsed_seconds = start.elapsed().as_secs_f32(),
                "waiting to confirm transaction inclusion"
            );
        }
    };

    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(MIN_POLL_INTERVAL)
        .max_delay(MAX_POLL_INTERVAL);

    let tx_fut = async move {
        tryhard::retry_fn(|| {
            let client = client.clone();
            let logged_at = logged_at.clone();
            let attempt_span = debug_span!("attempt get_tx");
            attempt_span.follows_from(Span::current());
            async move {
                match client.tx(tx_hash, false).await {
                    Ok(tx) => Ok(tx),
                    Err(error) => {
                        log_if_due(logged_at, error.to_string()).await;
                        Err(error)
                    }
                }
            }
            .instrument(attempt_span)
        })
        .with_config(retry_config)
        .await
    };

    time::timeout(Duration::from_secs(240), tx_fut)
        .await
        .wrap_err("timed out waiting for tx inclusion")?
        .wrap_err("failed to get tx")
}
