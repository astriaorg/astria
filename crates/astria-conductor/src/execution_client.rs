use astria_proto::generated::execution::v1alpha2::{
    execution_service_client::ExecutionServiceClient,
    BatchGetBlocksRequest,
    BatchGetBlocksResponse,
    Block,
    BlockIdentifier,
    CommitmentState,
    ExecuteBlockRequest,
    GetBlockRequest,
    GetCommitmentStateRequest,
    UpdateCommitmentStateRequest,
};
use color_eyre::eyre::Result;
use prost_types::Timestamp;
use tonic::transport::Channel;
use tracing::info;

/// Creates a new RPC Client for Execution
///
/// # Arguments
///
/// * `address` - The address of the RPC server that we want to communicate with.
pub(crate) async fn new(address: &str) -> Result<ExecutionServiceClient<Channel>> {
    let client = ExecutionServiceClient::connect(address.to_owned()).await?;
    info!("Connected to execution service at {}", address);
    Ok(client)
}

#[async_trait::async_trait]
pub(crate) trait ExecutionClient {
    async fn call_batch_get_blocks(
        &mut self,
        identifiers: Vec<BlockIdentifier>,
    ) -> Result<BatchGetBlocksResponse>;

    async fn call_execute_block(
        &mut self,
        prev_block_hash: Vec<u8>,
        transactions: Vec<Vec<u8>>,
        timestamp: Option<Timestamp>,
    ) -> Result<Block>;

    async fn call_get_block(&mut self, identifier: BlockIdentifier) -> Result<Block>;

    async fn call_get_commitment_state(&mut self) -> Result<CommitmentState>;

    async fn call_update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
    ) -> Result<CommitmentState>;
}

#[async_trait::async_trait]
impl ExecutionClient for ExecutionServiceClient<Channel> {
    /// Calls remote procedure BatchGetBlocks
    ///
    /// # Arguments
    ///
    /// * `identifiers` - List of block identifiers describes which blocks we want to get
    async fn call_batch_get_blocks(
        &mut self,
        identifiers: Vec<BlockIdentifier>,
    ) -> Result<BatchGetBlocksResponse> {
        let request = BatchGetBlocksRequest {
            identifiers,
        };
        let response = self.batch_get_blocks(request).await?.into_inner();
        Ok(response)
    }

    /// Calls remote procedure ExecuteBlock
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash` - Block hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    async fn call_execute_block(
        &mut self,
        prev_block_hash: Vec<u8>,
        transactions: Vec<Vec<u8>>,
        timestamp: Option<Timestamp>,
    ) -> Result<Block> {
        let request = ExecuteBlockRequest {
            prev_block_hash,
            transactions,
            timestamp,
        };
        let response = self.execute_block(request).await?.into_inner();
        Ok(response)
    }

    /// Calls remote procedure GetBlock
    ///
    /// # Arguments
    ///
    /// * `identifier` - The identifier describes the block we want to get
    async fn call_get_block(&mut self, identifier: BlockIdentifier) -> Result<Block> {
        let request = GetBlockRequest {
            identifier: Some(identifier),
        };
        let response = self.get_block(request).await?.into_inner();
        Ok(response)
    }

    /// Calls remote procedure GetCommitmentState
    async fn call_get_commitment_state(&mut self) -> Result<CommitmentState> {
        let request = GetCommitmentStateRequest {};
        let response = self.get_commitment_state(request).await?.into_inner();
        Ok(response)
    }

    /// Calls remote procedure UpdateCommitmentState
    ///
    /// # Arguments
    ///
    /// * `commitment_state` - The CommitmentState to set, must include complete state.
    async fn call_update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
    ) -> Result<CommitmentState> {
        let request = UpdateCommitmentStateRequest {
            commitment_state: Some(commitment_state),
        };
        let response = self.update_commitment_state(request).await?.into_inner();
        Ok(response)
    }
}
