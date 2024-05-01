use std::{
    sync::atomic::AtomicU32,
    time::Duration,
};

use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use celestia_types::{
    nmt::Namespace,
    Blob,
};
use jsonrpsee::{
    self,
    http_client::HttpClient as CelestiaClient,
};
use telemetry::display::base64;
use tokio::try_join;
use tracing::{
    instrument,
    warn,
};
use tryhard::{
    backoff_strategies::BackoffStrategy,
    RetryPolicy,
};

pub(super) struct RawBlobs {
    pub(super) celestia_height: u64,
    pub(super) header_blobs: Vec<Blob>,
    pub(super) rollup_blobs: Vec<Blob>,
}

impl RawBlobs {
    pub(super) fn len_header_blobs(&self) -> usize {
        self.header_blobs.len()
    }

    pub(super) fn len_rollup_blobs(&self) -> usize {
        self.rollup_blobs.len()
    }
}

/// Fetch Celestia blobs at `celestia_height` matching `sequencer_namespace` and `rollup_namespace`.
///
/// Retries indefinitely if the underlying transport failed. Immediately returns with an error in
/// all other cases.
#[instrument(skip_all, fields(
    celestia_height,
    sequencer_namespace = %base64(sequencer_namespace.as_ref()),
    rollup_namespace = %base64(rollup_namespace.as_ref()),
))]
pub(super) async fn fetch_new_blobs(
    client: CelestiaClient,
    celestia_height: u64,
    rollup_namespace: Namespace,
    sequencer_namespace: Namespace,
) -> eyre::Result<RawBlobs> {
    let header_blobs = async {
        fetch_blobs_with_retry(client.clone(), celestia_height, sequencer_namespace)
            .await
            .wrap_err("failed to fetch header blobs")
    };
    let rollup_blobs = async {
        fetch_blobs_with_retry(client.clone(), celestia_height, rollup_namespace)
            .await
            .wrap_err("failed to fetch rollup blobs")
    };

    let (header_blobs, rollup_blobs) = try_join!(header_blobs, rollup_blobs)?;

    Ok(RawBlobs {
        celestia_height,
        header_blobs,
        rollup_blobs,
    })
}

async fn fetch_blobs_with_retry(
    client: CelestiaClient,
    height: u64,
    namespace: Namespace,
) -> eyre::Result<Vec<Blob>> {
    use celestia_rpc::BlobClient as _;

    let number_attempts = AtomicU32::new(0);
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .custom_backoff(FetchBlobsRetryStrategy::new(Duration::from_millis(100)))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &jsonrpsee::core::Error| {
                number_attempts.store(attempt, std::sync::atomic::Ordering::Relaxed);
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch Celestia Blobs failed; retrying after delay",
                );
                futures::future::ready(())
            },
        );

    tryhard::retry_fn(move || {
        let client = client.clone();
        async move {
            match client.blob_get_all(height, &[namespace]).await {
                Ok(blobs) => Ok(blobs),
                Err(err) if is_blob_not_found(&err) => Ok(vec![]),
                Err(err) => Err(err),
            }
        }
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed fetching blocks without being able to recover")
}

struct FetchBlobsRetryStrategy {
    delay: Duration,
}

impl FetchBlobsRetryStrategy {
    fn new(initial_duration: Duration) -> Self {
        Self {
            delay: initial_duration,
        }
    }
}

impl<'a> BackoffStrategy<'a, jsonrpsee::core::Error> for FetchBlobsRetryStrategy {
    type Output = RetryPolicy;

    fn delay(&mut self, _attempt: u32, error: &'a jsonrpsee::core::Error) -> Self::Output {
        if should_retry(error) {
            let prev_delay = self.delay;
            self.delay = self.delay.saturating_mul(2);
            RetryPolicy::Delay(prev_delay)
        } else {
            RetryPolicy::Break
        }
    }
}

fn should_retry(error: &jsonrpsee::core::Error) -> bool {
    matches!(error, jsonrpsee::core::Error::Transport(_))
}

fn is_blob_not_found(error: &jsonrpsee::core::Error) -> bool {
    let jsonrpsee::core::Error::Call(error) = error else {
        return false;
    };
    error.code() == 1 && error.message().contains("blob: not found")
}
