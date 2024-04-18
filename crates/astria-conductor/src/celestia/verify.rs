use std::{
    sync::Arc,
    time::Duration,
};

use astria_eyre::{
    eyre,
    eyre::{
        ensure,
        WrapErr as _,
    },
};
use celestia_client::{
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
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
use tracing::{
    info,
    warn,
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
use crate::utils::flatten;

pub(super) struct VerifiedBlobs {
    celestia_height: u64,
    header_blobs: Vec<CelestiaSequencerBlob>,
    rollup_blobs: Vec<CelestiaRollupBlob>,
}

impl VerifiedBlobs {
    pub(super) fn len_header_blobs(&self) -> usize {
        self.header_blobs.len()
    }

    pub(super) fn len_rollup_blobs(&self) -> usize {
        self.rollup_blobs.len()
    }

    pub(super) fn into_parts(self) -> (u64, Vec<CelestiaSequencerBlob>, Vec<CelestiaRollupBlob>) {
        (self.celestia_height, self.header_blobs, self.rollup_blobs)
    }
}

/// Task key to track verification of multiple [`CelestiaSequencerBlob`]s.
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
pub(super) async fn verify_header_blobs(
    blob_verifier: Arc<BlobVerifier>,
    converted_blobs: ConvertedBlobs,
) -> VerifiedBlobs {
    let (celestia_height, header_blobs, rollup_blobs) = converted_blobs.into_parts();

    let mut verification_tasks = JoinMap::new();
    let mut verified_header_blobs = Vec::with_capacity(header_blobs.len());

    for (index, blob) in header_blobs.into_iter().enumerate() {
        verification_tasks.spawn(
            VerificationTaskKey {
                index,
                block_hash: blob.block_hash(),
                sequencer_height: blob.height(),
            },
            blob_verifier.clone().verify_header_blob(blob),
        );
    }

    while let Some((key, verification_result)) = verification_tasks.join_next().await {
        match flatten(verification_result) {
            Ok(verified_blob) => verified_header_blobs.push(verified_blob),
            Err(error) => {
                info!(
                    block_hash = %base64(&key.block_hash),
                    sequencer_height = %key.sequencer_height,
                    %error,
                    "verification of sequencer blob failed; dropping it"
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
    async fn fetch(
        client: SequencerClient,
        height: SequencerHeight,
    ) -> Result<Self, VerificationMetaError> {
        if height.value() == 0 {
            return Err(VerificationMetaError::CantVerifyHeightZero);
        }
        let prev_height = SequencerHeight::try_from(height.value().saturating_sub(1)).expect(
            "BUG: should always be able to convert a decremented cometbft height back to its \
             original type; if this is not the case then some fundamentals of cometbft or \
             tendermint-rs/cometbft-rs no longer hold (or this code has been running several \
             decades and the chain's height is greater u32::MAX)",
        );
        let (commit_response, validators_response) = tokio::try_join!(
            fetch_commit_with_retry(client.clone(), height),
            fetch_validators_with_retry(client.clone(), prev_height, height),
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
    sequencer_cometbft_client: SequencerClient,
}

impl BlobVerifier {
    pub(super) fn new(sequencer_cometbft_client: SequencerClient) -> Self {
        Self {
            // Cache for verifying 1_000 celestia heights, assuming 6 sequencer heights per Celestia
            // height
            cache: Cache::new(6_000),
            sequencer_cometbft_client,
        }
    }

    async fn verify_header_blob(
        self: Arc<Self>,
        blob: CelestiaSequencerBlob,
    ) -> eyre::Result<CelestiaSequencerBlob> {
        use base64::prelude::*;
        let height = blob.height();
        let meta = self
            .cache
            .try_get_with(
                height,
                VerificationMeta::fetch(self.sequencer_cometbft_client.clone(), height),
            )
            .await
            .wrap_err("failed getting data necessary to verify the sequencer header blob")?;
        ensure!(
            &meta.commit_header.header.chain_id == blob.cometbft_chain_id(),
            "expected cometbft chain ID `{}`, got `{}`",
            meta.commit_header.header.chain_id,
            blob.cometbft_chain_id(),
        );
        ensure!(
            meta.commit_header.commit.block_id.hash.as_bytes() == blob.block_hash(),
            "block hash `{}` stored in blob does not match block hash `{}` of sequencer block",
            BASE64_STANDARD.encode(blob.block_hash()),
            BASE64_STANDARD.encode(meta.commit_header.commit.block_id.hash.as_bytes()),
        );
        Ok(blob)
    }
}

async fn fetch_commit_with_retry(
    client: SequencerClient,
    height: SequencerHeight,
) -> Result<tendermint_rpc::endpoint::commit::Response, VerificationMetaError> {
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
    .map_err(|source| VerificationMetaError::FetchCommit {
        height,
        source,
    })
}

async fn fetch_validators_with_retry(
    client: SequencerClient,
    prev_height: SequencerHeight,
    height: SequencerHeight,
) -> Result<tendermint_rpc::endpoint::validators::Response, VerificationMetaError> {
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
