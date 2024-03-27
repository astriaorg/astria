use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::generated::execution::v1alpha2::{
    execution_service_server::{
        ExecutionService,
        ExecutionServiceServer,
    },
    BatchGetBlocksRequest,
    BatchGetBlocksResponse,
    Block,
    CommitmentState,
    ExecuteBlockRequest,
    GenesisInfo,
    GetBlockRequest,
    GetCommitmentStateRequest,
    GetGenesisInfoRequest,
    UpdateCommitmentStateRequest,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use astria_grpc_mock::MockServer;
use tokio::task::JoinHandle;
use tonic::{
    transport::Server,
    Request,
};

pub struct MockGrpc {
    pub _server: JoinHandle<eyre::Result<()>>,
    pub mock_server: MockServer,
    pub local_addr: SocketAddr,
}

impl MockGrpc {
    pub async fn spawn() -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let mock_server = MockServer::new();

        let server = {
            let execution_service = ExecutionServiceImpl::new(mock_server.clone());
            tokio::spawn(async move {
                Server::builder()
                    .add_service(ExecutionServiceServer::new(execution_service))
                    .serve_with_incoming(TcpListenerStream::new(listener))
                    .await
                    .wrap_err("gRPC server failed")
            })
        };
        Self {
            _server: server,
            mock_server,
            local_addr,
        }
    }
}

struct ExecutionServiceImpl {
    mock_server: MockServer,
}

impl ExecutionServiceImpl {
    fn new(mock_server: MockServer) -> Self {
        Self {
            mock_server,
        }
    }
}
#[tonic::async_trait]
impl ExecutionService for ExecutionServiceImpl {
    async fn get_block(
        self: Arc<Self>,
        request: Request<GetBlockRequest>,
    ) -> tonic::Result<tonic::Response<Block>> {
        self.mock_server.handle_request("get_block", request).await
    }

    async fn get_genesis_info(
        self: Arc<Self>,
        request: Request<GetGenesisInfoRequest>,
    ) -> tonic::Result<tonic::Response<GenesisInfo>> {
        self.mock_server
            .handle_request("get_genesis_info", request)
            .await
    }

    async fn batch_get_blocks(
        self: Arc<Self>,
        request: Request<BatchGetBlocksRequest>,
    ) -> tonic::Result<tonic::Response<BatchGetBlocksResponse>> {
        self.mock_server
            .handle_request("batch_get_blocks", request)
            .await
    }

    async fn execute_block(
        self: Arc<Self>,
        request: Request<ExecuteBlockRequest>,
    ) -> tonic::Result<tonic::Response<Block>> {
        self.mock_server
            .handle_request("execute_block", request)
            .await
    }

    async fn get_commitment_state(
        self: Arc<Self>,
        request: Request<GetCommitmentStateRequest>,
    ) -> tonic::Result<tonic::Response<CommitmentState>> {
        self.mock_server
            .handle_request("get_commitment_state", request)
            .await
    }

    async fn update_commitment_state(
        self: Arc<Self>,
        request: Request<UpdateCommitmentStateRequest>,
    ) -> tonic::Result<tonic::Response<CommitmentState>> {
        self.mock_server
            .handle_request("update_commitment_state", request)
            .await
    }
}
