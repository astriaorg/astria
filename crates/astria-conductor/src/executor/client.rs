use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    generated::execution::{
        v1alpha2 as raw,
        v1alpha2::execution_service_client::ExecutionServiceClient,
    },
    Protobuf as _,
};
use eyre::{
    self,
    WrapErr as _,
};
use prost_types::Timestamp;
use tonic::transport::Channel;
use tracing::instrument;

/// A newtype wrapper around [`ExecutionServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(crate) struct Client {
    uri: tonic::transport::Uri,
    inner: ExecutionServiceClient<Channel>,
}

impl Client {
    #[instrument(skip_all, fields(rollup_uri = %uri))]
    pub(crate) async fn connect(uri: tonic::transport::Uri) -> eyre::Result<Self> {
        let inner = ExecutionServiceClient::connect(uri.clone())
            .await
            .wrap_err("failed constructing execution service client")?;
        Ok(Self {
            uri,
            inner,
        })
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetGenesisInfo`
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_genesis_info(&mut self) -> eyre::Result<GenesisInfo> {
        let request = raw::GetGenesisInfoRequest {};
        let response = self
            .inner
            .get_genesis_info(request)
            .await
            .wrap_err("failed to get genesis_info")?
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
    #[instrument(skip_all, fields(uri = %self.uri))]
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
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_commitment_state(&mut self) -> eyre::Result<CommitmentState> {
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
    #[instrument(skip_all, fields(uri = %self.uri))]
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
