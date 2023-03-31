use astria_execution_apis_rpc::execution::execution_service_client::ExecutionServiceClient;
use astria_execution_apis_rpc::execution::{DoBlockRequest, DoBlockResponse, InitStateResponse, InitStateRequest};
use color_eyre::eyre::Result;
use prost_types::Timestamp;
use tonic::transport::Channel;

/// Represents an RpcClient. Wrapping the auto generated client here.
pub struct ExecutionRpcClient {
    /// The actual rpc client
    client: ExecutionServiceClient<Channel>,
}

impl ExecutionRpcClient {
    /// Creates a new RPC Client
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the RPC server that we want to communicate with.
    pub async fn new(address: &str) -> Result<Self> {
        let client = ExecutionServiceClient::connect(address.to_owned()).await?;
        Ok(ExecutionRpcClient { client })
    }

    /// Calls remote procedure DoBlock
    ///
    /// # Arguments
    ///
    /// * `header` - Header of the block
    /// * `transactions` - List of transactions
    pub async fn call_do_block(
        &mut self,
        prev_state_root: Vec<u8>,
        transactions: Vec<Vec<u8>>,
        timestamp: Option<Timestamp>
    ) -> Result<DoBlockResponse> {
        let request = DoBlockRequest {
            prev_state_root,
            transactions,
            timestamp,
        };
        let response = self.client.do_block(request).await?.into_inner();
        Ok(response)
    }

    /// Calls remote procedure InitState
    pub async fn call_init_state(&mut self) -> Result<InitStateResponse> {
        let request = InitStateRequest {};
        let response = self.client.init_state(request).await?.into_inner();
        Ok(response)
    }
}
