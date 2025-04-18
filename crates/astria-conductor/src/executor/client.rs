use std::time::Duration;

use astria_core::{
    execution::v2::{
        CommitmentState,
        ExecutedBlockMetadata,
        ExecutionSession,
    },
    generated::astria::{
        execution::v2::{
            self as raw,
            execution_service_client::ExecutionServiceClient,
        },
        sequencerblock::v1::RollupData,
    },
    sequencerblock::v1::block::Hash,
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    ensure,
    OptionExt as _,
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
use tryhard::{
    backoff_strategies::BackoffStrategy,
    RetryPolicy,
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

    /// Calls RPC astria.execution.v2.GetExecutedBlockMetadata
    #[instrument(skip_all, fields(block_number, uri = %self.uri), err)]
    pub(crate) async fn get_executed_block_metadata_with_retry(
        &mut self,
        block_number: u64,
    ) -> eyre::Result<ExecutedBlockMetadata> {
        let raw_block_metadata = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = raw::GetExecutedBlockMetadataRequest {
                identifier: Some(block_identifier(block_number)),
            };
            async move { client.get_executed_block_metadata(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v1.GetBlocks RPC because of gRPC status code or \
             because number of retries were exhausted",
        )?
        .into_inner();
        ensure!(
            block_number == raw_block_metadata.number,
            "requested block at number `{block_number}`, but received block contained `{}`",
            raw_block_metadata.number
        );
        ExecutedBlockMetadata::try_from_raw(raw_block_metadata)
            .wrap_err("failed validating received block")
    }

    /// Calls remote procedure `astria.execution.v2.CreateExecutionSession`
    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(crate) async fn create_execution_session_with_retry(
        &mut self,
    ) -> eyre::Result<ExecutionSession> {
        let response = tryhard::retry_fn(|| {
            let mut client = self.inner.clone();
            let request = raw::CreateExecutionSessionRequest {};
            async move { client.create_execution_session(request).await }
        })
        .with_config(retry_config())
        .in_current_span()
        .await
        .wrap_err(
            "failed to execute astria.execution.v2.CreateExecutionSession RPC because of gRPC \
             status code or because number of retries were exhausted",
        )?
        .into_inner();
        let execution_session = ExecutionSession::try_from_raw(response)
            .wrap_err("failed converting raw response to validated execution session")?;
        Ok(execution_session)
    }

    /// Calls remote procedure `astria.execution.v2.ExecuteBlock`.
    ///
    /// # Arguments
    ///
    /// * `session_id` - ID of the current execution session
    /// * `parent_hash` - Hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(super) async fn execute_block_with_retry(
        &mut self,
        session_id: String,
        parent_hash: String,
        transactions: Vec<Bytes>,
        timestamp: Timestamp,
        sequencer_block_hash: Hash,
    ) -> eyre::Result<ExecutedBlockMetadata> {
        use prost::Message;

        let transactions = transactions
            .into_iter()
            .map(RollupData::decode)
            .collect::<Result<_, _>>()
            .wrap_err("failed to decode tx bytes as RollupData")?;

        let request = raw::ExecuteBlockRequest {
            session_id,
            parent_hash,
            transactions,
            timestamp: Some(timestamp),
            sequencer_block_hash: sequencer_block_hash.to_string(),
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
            "failed to execute astria.execution.v2.ExecuteBlock RPC because of gRPC status code \
             or because number of retries were exhausted",
        )?
        .into_inner();
        let response_metadata = response
            .executed_block_metadata
            .ok_or_eyre("response is missing executed block metadata")?;
        let block_metadata = ExecutedBlockMetadata::try_from_raw(response_metadata)
            .wrap_err("failed converting raw response to validated block metadata")?;
        Ok(block_metadata)
    }

    /// Calls remote procedure `astria.execution.v2.UpdateCommitmentState`
    ///
    /// # Arguments
    ///
    /// * `session_id` - ID of the current execution session
    /// * `commitment_state` - New commitment state to be updated with
    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(super) async fn update_commitment_state_with_retry(
        &mut self,
        session_id: String,
        commitment_state: CommitmentState,
    ) -> eyre::Result<CommitmentState> {
        let request = raw::UpdateCommitmentStateRequest {
            session_id,
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
            "failed to execute astria.execution.v2.UpdateCommitmentState RPC because of gRPC \
             status code or because number of retries were exhausted",
        )?
        .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }
}

/// Utility function to construct a `astria.execution.v2.ExecutedBlockIdentifier` from `number`
/// to use in RPC requests.
fn block_identifier(number: u64) -> raw::ExecutedBlockIdentifier {
    raw::ExecutedBlockIdentifier {
        identifier: Some(raw::executed_block_identifier::Identifier::Number(number)),
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
            .map(telemetry::display::format_duration)
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

fn retry_config() -> tryhard::RetryFutureConfig<ExecutionApiRetryStrategy, OnRetry> {
    tryhard::RetryFutureConfig::new(u32::MAX)
        .custom_backoff(ExecutionApiRetryStrategy {
            delay: Duration::from_millis(100),
        })
        // XXX: This should probably be configurable.
        .max_delay(Duration::from_secs(10))
        .on_retry(OnRetry {
            parent: Span::current(),
        })
}

/// An exponential retry strategy branching on [`tonic::Status::code`].
///
/// This retry strategy behaves exactly like
/// [`tryhard::backoff_strategies::ExponentialBackoff`] but is specialized to
/// work with [`tonic::Status`].
///
/// Execution will be retried under the following conditions:
///
/// ```text
/// Code::Cancelled
/// Code::Unknown
/// Code::DeadlineExceeded
/// Code::NotFound
/// Code::ResourceExhausted
/// Code::Aborted
/// Code::Unavailable
/// ```
struct ExecutionApiRetryStrategy {
    delay: Duration,
}

impl<'a> BackoffStrategy<'a, tonic::Status> for ExecutionApiRetryStrategy {
    type Output = RetryPolicy;

    fn delay(&mut self, _attempt: u32, error: &'a tonic::Status) -> Self::Output {
        if should_retry(error) {
            let prev_delay = self.delay;
            self.delay = self.delay.saturating_mul(2);
            RetryPolicy::Delay(prev_delay)
        } else {
            RetryPolicy::Break
        }
    }
}

fn should_retry(status: &tonic::Status) -> bool {
    use tonic::Code;
    // gRPC return codes and if they should be retried. Also refer to
    // [1] https://github.com/grpc/grpc/blob/1309eb283c3e11c471191f286ceab01b75477ffc/doc/statuscodes.md
    //
    // Code::Ok => no, success
    // Code::Cancelled => yes, but should be revisited if "we" would cancel
    // Code::Unknown => yes, could this be returned if the endpoint is unavailable?
    // Code::InvalidArgument => no, no point retrying
    // Code::DeadlineExceeded => yes, server might be slow
    // Code::NotFound => yes, resource might not yet be available
    // Code::AlreadyExists => no, no point retrying
    // Code::PermissionDenied => no, execution API uses permission-denied restart-trigger
    // Code::ResourceExhausted => yes, retry after a while
    // Code::FailedPrecondition => no, failed precondition should not be retried unless the
    //                             precondition is fixed, see [1]
    // Code::Aborted => yes, although this applies to a read-modify-write sequence. We should
    //                  implement this not per-request but per-request-sequence (for example,
    //                  execute + update-commitment-state)
    // Code::OutOfRange => no, we don't expect to send any out-of-range requests.
    // Code::Unimplemented => no, no point retrying
    // Code::Internal => no, this is a serious error on the backend; don't retry
    // Code::Unavailable => yes, retry after backoff is desired
    // Code::DataLoss => no, unclear how this would happen, but sounds very terminal
    // Code::Unauthenticated => no, this status code will likely not change after retrying
    matches!(
        status.code(),
        Code::Cancelled
            | Code::Unknown
            | Code::DeadlineExceeded
            | Code::NotFound
            | Code::ResourceExhausted
            | Code::Aborted
            | Code::Unavailable
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tonic::{
        Code,
        Status,
    };

    use super::{
        BackoffStrategy as _,
        ExecutionApiRetryStrategy,
        RetryPolicy,
    };

    #[track_caller]
    fn assert_retry_policy<const SHOULD_RETRY: bool>(code: Code) {
        let mut strat = ExecutionApiRetryStrategy {
            delay: Duration::from_secs(1),
        };
        let status = Status::new(code, "");
        let actual = strat.delay(1, &status);
        if SHOULD_RETRY {
            let expected = RetryPolicy::Delay(Duration::from_secs(1));
            assert_eq!(
                expected, actual,
                "gRPC code `{code}` should lead to retry, but instead gave break"
            );
        } else {
            let expected = RetryPolicy::Break;
            assert_eq!(
                expected, actual,
                "gRPC code `{code}` should lead to break, but instead gave delay"
            );
        }
    }

    #[test]
    fn status_codes_lead_to_expected_retry_policy() {
        const SHOULD_RETRY: bool = true;
        const SHOULD_BREAK: bool = false;
        assert_retry_policy::<SHOULD_BREAK>(Code::Ok);
        assert_retry_policy::<SHOULD_RETRY>(Code::Cancelled);
        assert_retry_policy::<SHOULD_RETRY>(Code::Unknown);
        assert_retry_policy::<SHOULD_BREAK>(Code::InvalidArgument);
        assert_retry_policy::<SHOULD_RETRY>(Code::DeadlineExceeded);
        assert_retry_policy::<SHOULD_RETRY>(Code::NotFound);
        assert_retry_policy::<SHOULD_BREAK>(Code::AlreadyExists);
        assert_retry_policy::<SHOULD_BREAK>(Code::PermissionDenied);
        assert_retry_policy::<SHOULD_RETRY>(Code::ResourceExhausted);
        assert_retry_policy::<SHOULD_BREAK>(Code::FailedPrecondition);
        assert_retry_policy::<SHOULD_RETRY>(Code::Aborted);
        assert_retry_policy::<SHOULD_BREAK>(Code::OutOfRange);
        assert_retry_policy::<SHOULD_BREAK>(Code::Unimplemented);
        assert_retry_policy::<SHOULD_BREAK>(Code::Internal);
        assert_retry_policy::<SHOULD_RETRY>(Code::Unavailable);
        assert_retry_policy::<SHOULD_BREAK>(Code::DataLoss);
        assert_retry_policy::<SHOULD_BREAK>(Code::Unauthenticated);
    }
}
