use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use astria_core::sequencerblock::v1alpha1::{
    SubmittedMetadata,
    SubmittedRollupData,
};
use astria_eyre::{
    eyre,
    eyre::{
        ensure,
        WrapErr as _,
    },
};
use moka::future::Cache;
use sequencer_client::{
    tendermint::block::{
        signed_header::SignedHeader,
        Height as SequencerHeight,
    },
    tendermint_rpc,
    Client as _,
    HttpClient as SequencerClient,
};
use telemetry::display::base64;
use tokio_util::task::JoinMap;
use tower::{
    util::BoxService,
    BoxError,
    Service as _,
    ServiceExt as _,
};
use tracing::{
    info,
    instrument,
    warn,
    Instrument,
    Level,
};
use tryhard::{
    backoff_strategies::BackoffStrategy,
    retry_fn,
    RetryFutureConfig,
    RetryPolicy,
};

use super::{
    block_verifier,
    convert::ConvertedBlobs,
};
use crate::executor::{
    self,
    StateIsInit,
};

pub(super) struct VerifiedBlobs {
    celestia_height: u64,
    header_blobs: HashMap<[u8; 32], SubmittedMetadata>,
    rollup_blobs: Vec<SubmittedRollupData>,
}

impl VerifiedBlobs {
    pub(super) fn len_header_blobs(&self) -> usize {
        self.header_blobs.len()
    }

    pub(super) fn len_rollup_blobs(&self) -> usize {
        self.rollup_blobs.len()
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        u64,
        HashMap<[u8; 32], SubmittedMetadata>,
        Vec<SubmittedRollupData>,
    ) {
        (self.celestia_height, self.header_blobs, self.rollup_blobs)
    }
}

/// Task key to track verification of multiple [`SubmittedMetadata`] objects.
///
/// The index is necessary because two keys might have clashing hashes and heights.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct VerificationTaskKey {
    index: usize,
    block_hash: [u8; 32],
    sequencer_height: SequencerHeight,
}

/// Verifies Sequencer header blobs against Sequencer commits and validator sets.
///
/// Drops blobs that could not be verified.
#[instrument(skip_all)]
pub(super) async fn verify_metadata(
    blob_verifier: Arc<BlobVerifier>,
    converted_blobs: ConvertedBlobs,
    mut executor: executor::Handle<StateIsInit>,
) -> VerifiedBlobs {
    let (celestia_height, header_blobs, rollup_blobs) = converted_blobs.into_parts();

    let mut verification_tasks = JoinMap::new();
    let mut verified_header_blobs = HashMap::with_capacity(header_blobs.len());

    let next_expected_firm_sequencer_height =
        executor.next_expected_firm_sequencer_height().value();

    for (index, blob) in header_blobs.into_iter().enumerate() {
        if blob.height().value() < next_expected_firm_sequencer_height {
            info!(
                next_expected_firm_sequencer_height,
                sequencer_height_in_metadata = blob.height().value(),
                "dropping Sequencer metadata item without verifying against Sequencer because its \
                 height is below the next expected firm height"
            );
        } else {
            verification_tasks.spawn(
                VerificationTaskKey {
                    index,
                    block_hash: *blob.block_hash(),
                    sequencer_height: blob.height(),
                },
                blob_verifier
                    .clone()
                    .verify_metadata(blob)
                    .in_current_span(),
            );
        }
    }

    while let Some((key, verification_result)) = verification_tasks.join_next().await {
        match verification_result {
            Ok(Some(verified_blob)) => {
                if let Some(dropped_entry) =
                    verified_header_blobs.insert(*verified_blob.block_hash(), verified_blob)
                {
                    let accepted_entry = verified_header_blobs
                        .get(dropped_entry.block_hash())
                        .expect("must exist; just inserted an item under the same key");
                    info!(
                        block_hash = %base64(&dropped_entry.block_hash()),
                        dropped_blob.sequencer_height = dropped_entry.height().value(),
                        accepted_blob.sequencer_height = accepted_entry.height().value(),
                        "two Sequencer header blobs were well formed and validated against \
                         Sequencer, but shared the same block hash, potentially duplicates? \
                         Dropping one",
                    );
                }
            }
            Ok(None) => {}
            Err(error) => {
                info!(
                    block_hash = %base64(&key.block_hash),
                    sequencer_height = %key.sequencer_height,
                    %error,
                    "verification of sequencer blob was cancelled abruptly; dropping it"
                );
            }
        }
    }

    VerifiedBlobs {
        celestia_height,
        header_blobs: verified_header_blobs,
        rollup_blobs,
    }
}

#[derive(Debug, thiserror::Error)]
enum VerificationMetaError {
    #[error("cannot verify a sequencer height zero")]
    CantVerifyHeightZero,
    #[error("failed fetching sequencer block commit for height {height}")]
    FetchCommit {
        height: SequencerHeight,
        source: tendermint_rpc::Error,
    },
    #[error(
        "failed fetching Sequencer validators at height `{prev_height}` (to validate a \
         Celestia-derived Sequencer block at height `{height}`)"
    )]
    FetchValidators {
        prev_height: SequencerHeight,
        height: SequencerHeight,
        source: tendermint_rpc::Error,
    },

    #[error(
        "failed ensuring quorum for commit at height `{height_of_commit}` using validator set at \
         height `{height_of_validator_set}`"
    )]
    NoQuorum {
        height_of_commit: SequencerHeight,
        height_of_validator_set: SequencerHeight,
        source: block_verifier::QuorumError,
    },
}

/// Data required to verify a
#[derive(Clone, Debug)]
struct VerificationMeta {
    commit_header: SignedHeader,
}

impl VerificationMeta {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn fetch(
        client: RateLimitedVerificationClient,
        height: SequencerHeight,
    ) -> Result<Self, BoxError> {
        if height.value() == 0 {
            return Err(VerificationMetaError::CantVerifyHeightZero.into());
        }
        let prev_height = SequencerHeight::try_from(height.value().saturating_sub(1)).expect(
            "BUG: should always be able to convert a decremented cometbft height back to its \
             original type; if this is not the case then some fundamentals of cometbft or \
             tendermint-rs/cometbft-rs no longer hold (or this code has been running several \
             decades and the chain's height is greater u32::MAX)",
        );
        let (commit_response, validators_response) = tokio::try_join!(
            client.clone().get_commit(height),
            client.clone().get_validators(prev_height, height),
        )?;
        super::ensure_commit_has_quorum(
            &commit_response.signed_header.commit,
            &validators_response,
            &commit_response.signed_header.header.chain_id,
        )
        .map_err(|source| VerificationMetaError::NoQuorum {
            height_of_commit: height,
            height_of_validator_set: prev_height,
            source,
        })?;

        Ok(Self {
            commit_header: commit_response.signed_header,
        })
    }
}

pub(super) struct BlobVerifier {
    cache: Cache<SequencerHeight, VerificationMeta>,
    client: RateLimitedVerificationClient,
}

impl BlobVerifier {
    pub(super) fn try_new(
        client: SequencerClient,
        requests_per_seconds: u32,
    ) -> eyre::Result<Self> {
        Ok(Self {
            // Cache for verifying 1_000 celestia heights, assuming 6 sequencer heights per Celestia
            // height
            cache: Cache::new(6_000),
            client: RateLimitedVerificationClient::try_new(client, requests_per_seconds)
                .wrap_err("failed to construct Sequencer block client")?,
        })
    }

    /// Verifies `metadata` against a remote Sequencer CometBFT instance.
    ///
    /// *Implementation note:* because [`Cache::try_get_with`] returns an
    /// `Arc<BoxError>` in error position (due to [`RateLimitedVerificationClient`]),
    /// this method cannot return an `eyre::Result` but needs to emit all errors
    /// it encounters.
    #[instrument(skip_all)]
    async fn verify_metadata(
        self: Arc<Self>,
        metadata: SubmittedMetadata,
    ) -> Option<SubmittedMetadata> {
        let height = metadata.height();
        let cached = self
            .cache
            .try_get_with(height, VerificationMeta::fetch(self.client.clone(), height))
            .await
            .inspect_err(|e| {
                warn!(
                    error = %e.as_ref(),
                    "failed getting data necessary to verify the sequencer metadata retrieved from Celestia",
                );
            })
            .ok()?;
        if let Err(error) = ensure_chain_ids_match(
            cached.commit_header.header.chain_id.as_str(),
            metadata.cometbft_chain_id().as_str(),
        )
        .and_then(|()| {
            ensure_block_hashes_match(
                cached.commit_header.commit.block_id.hash.as_bytes(),
                metadata.block_hash(),
            )
        }) {
            info!(reason = %error, "failed to verify metadata retrieved from Celestia; dropping it");
        }
        Some(metadata)
    }
}

#[instrument(skip_all, err)]
async fn fetch_commit_with_retry(
    client: SequencerClient,
    height: SequencerHeight,
) -> Result<VerificationResponse, VerificationMetaError> {
    let retry_config = RetryFutureConfig::new(u32::MAX)
        .custom_backoff(CometBftRetryStrategy::new(Duration::from_millis(100)))
        .max_delay(Duration::from_secs(10))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch Sequencer validators failed; retrying after delay",
                );
                futures::future::ready(())
            },
        );
    retry_fn(move || {
        let client = client.clone();
        async move { client.commit(height).await }
    })
    .with_config(retry_config)
    .await
    .map(Into::into)
    .map_err(|source| VerificationMetaError::FetchCommit {
        height,
        source,
    })
}

#[instrument(skip_all, err)]
async fn fetch_validators_with_retry(
    client: SequencerClient,
    prev_height: SequencerHeight,
    height: SequencerHeight,
) -> Result<VerificationResponse, VerificationMetaError> {
    let retry_config = RetryFutureConfig::new(u32::MAX)
        .custom_backoff(CometBftRetryStrategy::new(Duration::from_millis(100)))
        .max_delay(Duration::from_secs(10))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch Sequencer validators failed; retrying after delay",
                );
                futures::future::ready(())
            },
        );
    retry_fn(move || {
        let client = client.clone();
        async move {
            client
                .validators(prev_height, tendermint_rpc::Paging::Default)
                .await
        }
    })
    .with_config(retry_config)
    .await
    .map(Into::into)
    .map_err(|source| VerificationMetaError::FetchValidators {
        height,
        prev_height,
        source,
    })
}

struct CometBftRetryStrategy {
    delay: Duration,
}

impl CometBftRetryStrategy {
    fn new(initial_duration: Duration) -> Self {
        Self {
            delay: initial_duration,
        }
    }
}

impl<'a> BackoffStrategy<'a, tendermint_rpc::Error> for CometBftRetryStrategy {
    type Output = RetryPolicy;

    fn delay(&mut self, _attempt: u32, error: &'a tendermint_rpc::Error) -> Self::Output {
        if should_retry(error) {
            let prev_delay = self.delay;
            self.delay = self.delay.saturating_mul(2);
            RetryPolicy::Delay(prev_delay)
        } else {
            RetryPolicy::Break
        }
    }
}

fn should_retry(error: &tendermint_rpc::Error) -> bool {
    use tendermint_rpc::error::ErrorDetail::{
        Http,
        HttpRequestFailed,
        Timeout,
    };
    matches!(
        error.detail(),
        Http(..) | HttpRequestFailed(..) | Timeout(..)
    )
}

enum VerificationRequest {
    Commit {
        height: SequencerHeight,
    },
    Validators {
        prev_height: SequencerHeight,
        height: SequencerHeight,
    },
}

#[derive(Debug)]
enum VerificationResponse {
    Commit(Box<tendermint_rpc::endpoint::commit::Response>),
    Validators(Box<tendermint_rpc::endpoint::validators::Response>),
}

impl From<tendermint_rpc::endpoint::commit::Response> for VerificationResponse {
    fn from(value: tendermint_rpc::endpoint::commit::Response) -> Self {
        Self::Commit(Box::new(value))
    }
}

impl From<tendermint_rpc::endpoint::validators::Response> for VerificationResponse {
    fn from(value: tendermint_rpc::endpoint::validators::Response) -> Self {
        Self::Validators(Box::new(value))
    }
}

#[derive(Clone)]
struct RateLimitedVerificationClient {
    inner: tower::buffer::Buffer<
        BoxService<VerificationRequest, VerificationResponse, VerificationMetaError>,
        VerificationRequest,
    >,
}

impl RateLimitedVerificationClient {
    #[instrument(skip_all, err)]
    async fn get_commit(
        mut self,
        height: SequencerHeight,
    ) -> Result<Box<tendermint_rpc::endpoint::commit::Response>, BoxError> {
        // allow: it is desired that the wildcard matches all future added variants because
        // this call must only return a single specific variant, panicking otherwise.
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self
            .inner
            .ready()
            .await?
            .call(VerificationRequest::Commit {
                height,
            })
            .await?
        {
            VerificationResponse::Commit(commit) => Ok(commit),
            other => panic!("expected VerificationResponse::Commit, got {other:?}"),
        }
    }

    #[instrument(skip_all, err)]
    async fn get_validators(
        mut self,
        prev_height: SequencerHeight,
        height: SequencerHeight,
    ) -> Result<Box<tendermint_rpc::endpoint::validators::Response>, BoxError> {
        // allow: it is desired that the wildcard matches all future added variants because
        // this call must only return a single specific variant, panicking otherwise.
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self
            .inner
            .ready()
            .await?
            .call(VerificationRequest::Validators {
                prev_height,
                height,
            })
            .await?
        {
            VerificationResponse::Validators(validators) => Ok(validators),
            other => panic!("expected VerificationResponse::Validators, got {other:?}"),
        }
    }

    fn try_new(client: SequencerClient, requests_per_second: u32) -> eyre::Result<Self> {
        // XXX: the construction in here is a bit strange:
        // the straight forward way to create a type-erased tower service is to use
        // ServiceBuilder::boxed_clone().
        //
        // However, this gives a BoxCloneService which is always Clone + Send, but !Sync.
        // Therefore we can't use the ServiceBuilder::buffer adapter.
        //
        // We can however work around it: ServiceBuilder::boxed gives a BoxService, which is
        // Send + Sync, but not Clone. We then manually evoke Buffer::new to create a
        // Buffer<BoxService>, which is Send + Sync + Clone.
        let service = tower::ServiceBuilder::new()
            .boxed()
            .rate_limit(requests_per_second.into(), Duration::from_secs(1))
            .service_fn(move |req: VerificationRequest| {
                let client = client.clone();
                async move {
                    match req {
                        VerificationRequest::Commit {
                            height,
                        } => fetch_commit_with_retry(client, height).await,
                        VerificationRequest::Validators {
                            prev_height,
                            height,
                        } => fetch_validators_with_retry(client, prev_height, height).await,
                    }
                }
            });
        // XXX: This number is arbitarily set to the same number os the rate-limit. Does that
        // make sense? Should the number be set higher?
        let inner = tower::buffer::Buffer::new(
            service,
            requests_per_second
                .try_into()
                .wrap_err("failed to convert u32 requests-per-second to usize")?,
        );
        Ok(Self {
            inner,
        })
    }
}

fn ensure_chain_ids_match(in_commit: &str, in_header: &str) -> eyre::Result<()> {
    ensure!(
        in_commit == in_header,
        "expected cometbft chain ID `{in_commit}` (from commit), but found `{in_header}` in \
         retrieved metadata"
    );
    Ok(())
}

fn ensure_block_hashes_match(in_commit: &[u8], in_header: &[u8]) -> eyre::Result<()> {
    use base64::prelude::*;
    ensure!(
        in_commit == in_header,
        "expected block hash `{}` (from commit), but found `{}` in retrieved metadata",
        BASE64_STANDARD.encode(in_commit),
        BASE64_STANDARD.encode(in_header),
    );
    Ok(())
}
