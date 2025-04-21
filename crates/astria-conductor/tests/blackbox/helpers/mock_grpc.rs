use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::generated::{
    astria::{
        execution::v2::{
            execution_service_server::{
                ExecutionService,
                ExecutionServiceServer,
            },
            CommitmentState,
            CreateExecutionSessionRequest,
            ExecuteBlockRequest,
            ExecuteBlockResponse,
            ExecutedBlockMetadata,
            ExecutionSession,
            GetExecutedBlockMetadataRequest,
            UpdateCommitmentStateRequest,
        },
        sequencerblock::v1::{
            sequencer_service_server::{
                SequencerService,
                SequencerServiceServer,
            },
            FilteredSequencerBlock,
            GetFilteredSequencerBlockRequest,
            GetPendingNonceRequest,
            GetPendingNonceResponse,
            GetSequencerBlockRequest,
            SequencerBlock,
        },
    },
    sequencerblock::v1::{
        GetUpgradesInfoRequest,
        GetUpgradesInfoResponse,
        GetValidatorNameRequest,
        GetValidatorNameResponse,
    },
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
    Response,
};

pub struct MockGrpc {
    _server: JoinHandle<eyre::Result<()>>,
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
            let sequencer_service = SequencerServiceImpl::new(mock_server.clone());
            tokio::spawn(async move {
                Server::builder()
                    .add_service(ExecutionServiceServer::new(execution_service))
                    .add_service(SequencerServiceServer::new(sequencer_service))
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

struct SequencerServiceImpl {
    mock_server: MockServer,
}

impl SequencerServiceImpl {
    fn new(mock_server: MockServer) -> Self {
        Self {
            mock_server,
        }
    }
}

// XXX: Manually implementing this trait instead of using the `define_and_impl_service!` macro
// because `GetSequencerBlockRequest` and `SequencerBlock` don't currently implement
// `serde::Serialize`.
#[tonic::async_trait]
impl SequencerService for SequencerServiceImpl {
    async fn get_sequencer_block(
        self: Arc<Self>,
        _request: Request<GetSequencerBlockRequest>,
    ) -> tonic::Result<Response<SequencerBlock>> {
        unimplemented!()
    }

    async fn get_filtered_sequencer_block(
        self: Arc<Self>,
        request: Request<GetFilteredSequencerBlockRequest>,
    ) -> tonic::Result<Response<FilteredSequencerBlock>> {
        self.mock_server
            .handle_request("get_filtered_sequencer_block", request)
            .await
    }

    async fn get_pending_nonce(
        self: Arc<Self>,
        _request: Request<GetPendingNonceRequest>,
    ) -> tonic::Result<Response<GetPendingNonceResponse>> {
        unimplemented!()
    }

    async fn get_upgrades_info(
        self: Arc<Self>,
        _request: Request<GetUpgradesInfoRequest>,
    ) -> tonic::Result<Response<GetUpgradesInfoResponse>> {
        unimplemented!()
    }

    async fn get_validator_name(
        self: Arc<Self>,
        _request: Request<GetValidatorNameRequest>,
    ) -> tonic::Result<Response<GetValidatorNameResponse>> {
        unimplemented!()
    }
}

macro_rules! define_and_impl_service {
    (impl $trait:ident for $target:ident { $( ($rpc:ident: $request:ty => $response:ty) )* }) => {
        struct $target {
            mock_server: ::astria_grpc_mock::MockServer,
        }

        impl $target {
            fn new(mock_server: ::astria_grpc_mock::MockServer) -> Self {
                Self { mock_server, }
            }
        }

        #[tonic::async_trait]
        impl $trait for $target {
            $(
            async fn $rpc(self: Arc<Self>, request: ::tonic::Request<$request>) -> ::tonic::Result<::tonic::Response<$response>> {
                    self.mock_server.handle_request(stringify!($rpc), request).await
            }
            )+
        }
    }
}

define_and_impl_service!(impl ExecutionService for ExecutionServiceImpl {
    (get_executed_block_metadata: GetExecutedBlockMetadataRequest => ExecutedBlockMetadata)
    (create_execution_session: CreateExecutionSessionRequest => ExecutionSession)
    (execute_block: ExecuteBlockRequest => ExecuteBlockResponse)
    (update_commitment_state: UpdateCommitmentStateRequest => CommitmentState)
});
