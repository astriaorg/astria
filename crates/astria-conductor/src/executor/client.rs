use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
    },
    generated::execution::{
        v1alpha2 as raw,
        v1alpha2::execution_service_client::ExecutionServiceClient,
    },
    Protobuf as _,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use prost_types::Timestamp;
use tonic::transport::Channel;

/// A newtype wrapper around [`ExecutionServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(super) struct Client {
    inner: ExecutionServiceClient<Channel>,
}

impl Client {
    pub(super) fn from_execution_service_client(inner: ExecutionServiceClient<Channel>) -> Self {
        Self {
            inner,
        }
    }

    /// Calls remote procedure `astria.execution.v1alpha2.ExecuteBlock`
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash` - Block hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    pub(super) async fn execute_block(
        &mut self,
        prev_block_hash: [u8; 32],
        transactions: Vec<Vec<u8>>,
        timestamp: Timestamp,
    ) -> eyre::Result<Block> {
        let request = raw::ExecuteBlockRequest {
            prev_block_hash: prev_block_hash.to_vec(),
            transactions,
            timestamp: Some(timestamp),
        };
        let response = self
            .inner
            .execute_block(request)
            .await
            .wrap_err("failed to execute block")?
            .into_inner();
        let block = Block::try_from_raw(response)
            .wrap_err("failed converting raw response to validated block")?;
        Ok(block)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetCommitmentState`
    pub(super) async fn get_commitment_state(&mut self) -> eyre::Result<CommitmentState> {
        let request = raw::GetCommitmentStateRequest {};
        let response = self
            .inner
            .get_commitment_state(request)
            .await
            .wrap_err("failed to get commitment state")?
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
    pub(super) async fn update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
    ) -> eyre::Result<CommitmentState> {
        let request = raw::UpdateCommitmentStateRequest {
            commitment_state: Some(commitment_state.into_raw()),
        };
        let response = self
            .inner
            .update_commitment_state(request)
            .await
            .wrap_err("failed to update commitment state")?
            .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }
}
