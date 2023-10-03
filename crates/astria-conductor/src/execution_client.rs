use astria_proto::generated::execution::v1alpha1::{
    execution_service_client::ExecutionServiceClient,
    DoBlockRequest,
    DoBlockResponse,
    FinalizeBlockRequest,
    InitStateRequest,
    InitStateResponse,
};
use color_eyre::eyre::Result;
use prost_types::Timestamp;
use tonic::transport::Channel;
use tracing::{
    info,
    instrument,
};

#[async_trait::async_trait]
pub(crate) trait ExecutionClient {
    async fn call_do_block(
        &mut self,
        prev_block_hash: Vec<u8>,
        transactions: Vec<Vec<u8>>,
        timestamp: Option<Timestamp>,
    ) -> Result<DoBlockResponse>;

    async fn call_finalize_block(&mut self, block_hash: Vec<u8>) -> Result<()>;

    async fn call_init_state(&mut self) -> Result<InitStateResponse>;
}

/// Represents an RpcClient. Wrapping the auto generated client here.
pub(crate) struct ExecutionRpcClient {
    /// The actual rpc client
    client: ExecutionServiceClient<Channel>,
}

impl ExecutionRpcClient {
    /// Creates a new RPC Client
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the RPC server that we want to communicate with.
    pub(crate) async fn new(address: &str) -> Result<Self> {
        let client = ExecutionServiceClient::connect(address.to_owned()).await?;
        info!("Connected to execution service at {}", address);
        Ok(ExecutionRpcClient {
            client,
        })
    }
}

#[async_trait::async_trait]
impl ExecutionClient for ExecutionRpcClient {
    /// Calls remote procedure DoBlock
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash` - Block hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    #[instrument(skip_all)]
    async fn call_do_block(
        &mut self,
        prev_block_hash: Vec<u8>,
        transactions: Vec<Vec<u8>>,
        timestamp: Option<Timestamp>,
    ) -> Result<DoBlockResponse> {
        let request = DoBlockRequest {
            prev_block_hash,
            transactions,
            timestamp,
        };
        info!(do_block_request = ?request, "Sending request to execution service");
        let response = self.client.do_block(request).await?.into_inner();
        info!(hash = ?response , "Received response from execution service");
        Ok(response)
    }

    /// Calls remote procedure FinalizeBlock
    #[instrument(skip_all)]
    async fn call_finalize_block(&mut self, block_hash: Vec<u8>) -> Result<()> {
        let request = FinalizeBlockRequest {
            block_hash,
        };
        info!(request = ?request, "Sending finalize block request to execution service");
        self.client.finalize_block(request).await?;
        info!("Finalized block call from execution service completed");
        Ok(())
    }

    /// Calls remote procedure InitState
    #[instrument(skip_all)]
    async fn call_init_state(&mut self) -> Result<InitStateResponse> {
        let request = InitStateRequest {};
        info!("Sending init state request to execution service");
        let response = self.client.init_state(request).await?.into_inner();
        info!(response = ?response, "Received init state response from execution service");
        Ok(response)
    }
}
