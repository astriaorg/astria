use std::time::Duration;

use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    generated::{
        execution::{
            v1alpha2 as raw,
            v1alpha2::execution_service_client::ExecutionServiceClient,
        },
        sequencerblock::v1alpha1::RollupData,
    },
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use bytes::Bytes;
use pbjson_types::Timestamp;
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    instrument,
    warn,
    Instrument,
    Span,
};

/// A newtype wrapper around [`ExecutionServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(crate) struct Client {
    uri: Uri,
    inner: ExecutionServiceClient<Channel>,
}

impl Client {
    pub(crate) fn connect_lazy(uri: &str) -> eyre::Result<Self> {
        let uri: Uri = uri
            .parse()
            .wrap_err("failed to parse provided string as uri")?;
        let endpoint = Endpoint::from(uri.clone()).connect_lazy();
        let inner = ExecutionServiceClient::new(endpoint);
        Ok(Self {
            uri,
            inner,
        })
    }

    /// Calls RPC astria.execution.v1alpha2.GetBlock
    #[instrument(skip_all, fields(block_number, uri = %self.uri), err)]
    pub(crate) async fn get_block_with_retry(&mut self, block_number: u32) -> eyre::Result<Block> {
        let raw_block = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = raw::GetBlockRequest {
                identifier: Some(block_identifier(block_number)),
            };
            async move { client.get_block(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1alpha2.GetBlocks RPC after multiple retries; \
             giving up",
        )?
        .into_inner();
        ensure!(
            block_number == raw_block.number,
            "requested block at number `{block_number}`, but received block contained `{}`",
            raw_block.number
        );
        Block::try_from_raw(raw_block).wrap_err("failed validating received block")
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetGenesisInfo`
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_genesis_info_with_retry(&mut self) -> eyre::Result<GenesisInfo> {
        let response = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = raw::GetGenesisInfoRequest {};
            async move { client.get_genesis_info(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1alpha2.GetGenesisInfo RPC after multiple \
             retries; giving up",
        )?
        .into_inner();
        let genesis_info = GenesisInfo::try_from_raw(response)
            .wrap_err("failed converting raw response to validated genesis info")?;
        Ok(genesis_info)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.ExecuteBlock`
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash` - Block hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(super) async fn execute_block_with_retry(
        &mut self,
        prev_block_hash: Bytes,
        transactions: Vec<Vec<u8>>,
        timestamp: Timestamp,
    ) -> eyre::Result<Block> {
        use prost::Message;

        let transactions = transactions
            .into_iter()
            .map(|tx| RollupData::decode(tx.as_slice()))
            .collect::<Result<_, _>>()
            .wrap_err("failed to decode tx bytes as RollupData")?;

        let request = raw::ExecuteBlockRequest {
            prev_block_hash,
            transactions,
            timestamp: Some(timestamp),
        };
        let response = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = request.clone();
            async move { client.execute_block(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1alpha2.ExecuteBlock RPC after multiple retries; \
             giving up",
        )?
        .into_inner();
        let block = Block::try_from_raw(response)
            .wrap_err("failed converting raw response to validated block")?;
        Ok(block)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetCommitmentState`
    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(crate) async fn get_commitment_state_with_retry(
        &mut self,
    ) -> eyre::Result<CommitmentState> {
        let response = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            async move {
                let request = raw::GetCommitmentStateRequest {};
                client.get_commitment_state(request).await
            }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1alpha2.GetCommitmentState RPC after multiple \
             retries; giving up",
        )?
        .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.UpdateCommitmentState`
    ///
    /// # Arguments
    ///
    /// * `firm` - The firm block
    /// * `soft` - The soft block
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(super) async fn update_commitment_state_with_retry(
        &mut self,
        commitment_state: CommitmentState,
    ) -> eyre::Result<CommitmentState> {
        let request = raw::UpdateCommitmentStateRequest {
            commitment_state: Some(commitment_state.into_raw()),
        };
        let response = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = request.clone();
            async move { client.update_commitment_state(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1alpha2.UpdateCommitmentState RPC after multiple \
             retries; giving up",
        )?
        .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }
}

/// Utility function to construct a `astria.execution.v1alpha2.BlockIdentifier` from `number`
/// to use in RPC requests.
fn block_identifier(number: u32) -> raw::BlockIdentifier {
    raw::BlockIdentifier {
        identifier: Some(raw::block_identifier::Identifier::BlockNumber(number)),
    }
}

struct OnRetry {
    parent: Span,
}

impl tryhard::OnRetry<tonic::Status> for OnRetry {
    type Future = futures::future::Ready<()>;

    fn on_retry(
        &mut self,
        attempt: u32,
        next_delay: Option<Duration>,
        previous_error: &tonic::Status,
    ) -> Self::Future {
        let wait_duration = next_delay
            .map(humantime::format_duration)
            .map(tracing::field::display);
        warn!(
            parent: self.parent.id(),
            attempt,
            wait_duration,
            error = previous_error as &dyn std::error::Error,
            "failed executing RPC; retrying after after backoff"
        );
        futures::future::ready(())
    }
}

fn retry_config()
-> tryhard::RetryFutureConfig<tryhard::backoff_strategies::ExponentialBackoff, OnRetry> {
    tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        // XXX: This should probably be configurable.
        .max_delay(Duration::from_secs(10))
        .on_retry(OnRetry {
            parent: Span::current(),
        })
}
