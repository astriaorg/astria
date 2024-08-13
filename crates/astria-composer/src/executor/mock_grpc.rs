#![allow(dead_code)]
use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::generated::{
    execution::v1alpha2::{
        execution_service_server::{
            ExecutionService,
            ExecutionServiceServer,
        },
        BatchGetBlocksRequest,
        BatchGetBlocksResponse,
        Block,
        CommitmentState,
        ExecuteBlockRequest,
        ExecuteBlockResponse,
        GenesisInfo,
        GetBlockRequest,
        GetCommitmentStateRequest,
        GetGenesisInfoRequest,
        UpdateCommitmentStateRequest,
    },
    sequencerblock::v1alpha1::{
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
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use astria_grpc_mock::{
    AnyMessage,
    Match,
    MockServer,
};
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
    (execute_block: ExecuteBlockRequest => ExecuteBlockResponse)
    (get_commitment_state: GetCommitmentStateRequest => CommitmentState)
    (get_block: GetBlockRequest => Block)
    (get_genesis_info: GetGenesisInfoRequest => GenesisInfo)
    (batch_get_blocks: BatchGetBlocksRequest => BatchGetBlocksResponse)
    (update_commitment_state: UpdateCommitmentStateRequest => CommitmentState)
});

#[macro_export]
macro_rules! execute_block_response {
    (number: $number:expr,hash: $hash:expr,parent: $parent:expr $(,)?, included_transactions: $included_transactions:expr $(,)?) => {
        ::astria_core::generated::execution::v1alpha2::ExecuteBlockResponse {
            block: Some($crate::block!(
                number: $number,
                hash: $hash,
                parent: $parent,
            )),
            included_transactions: $included_transactions,
        }
    };
}

#[macro_export]
macro_rules! block {
    (number: $number:expr,hash: $hash:expr,parent: $parent:expr $(,)?) => {
        ::astria_core::generated::execution::v1alpha2::Block {
            number: $number,
            hash: ::bytes::Bytes::from(Vec::from($hash)),
            parent_block_hash: ::bytes::Bytes::from(Vec::from($parent)),
            timestamp: Some(::pbjson_types::Timestamp {
                seconds: 1,
                nanos: 1,
            }),
        }
    };
}

#[macro_export]
macro_rules! commitment_state {
    (
        firm: (number: $firm_number:expr,hash: $firm_hash:expr,parent: $firm_parent:expr $(,)?),
        soft: (number: $soft_number:expr,hash: $soft_hash:expr,parent: $soft_parent:expr $(,)?),
        base_celestia_height: $base_celestia_height:expr $(,)?
    ) => {
       ::astria_core::generated::execution::v1alpha2::CommitmentState {
            firm: Some($crate::block!(
                number: $firm_number,
                hash: $firm_hash,
                parent: $firm_parent,
            )),
            soft: Some($crate::block!(
                number: $soft_number,
                hash: $soft_hash,
                parent: $soft_parent,
            )),
           base_celestia_height: $base_celestia_height,
        }
    };
}

#[macro_export]
macro_rules! mount_get_commitment_state {
    (
        $test_env:ident,
        firm: ( number: $firm_number:expr, hash: $firm_hash:expr, parent: $firm_parent:expr$(,)? ),
        soft: ( number: $soft_number:expr, hash: $soft_hash:expr, parent: $soft_parent:expr$(,)? ),
        base_celestia_height: $base_celestia_height:expr
        $(,)?
    ) => {
        $test_env
            .mount_get_commitment_state($crate::commitment_state!(
                firm: (
                    number: $firm_number,
                    hash: $firm_hash,
                    parent: $firm_parent,
                ),
                soft: (
                    number: $soft_number,
                    hash: $soft_hash,
                    parent: $soft_parent,
                ),
                base_celestia_height: $base_celestia_height,
            ))
        .await
    };
}

#[macro_export]
macro_rules! mount_executed_block {
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
        hash: $hash:expr,
        included_transactions: $included_transactions:expr,
        parent: $parent:expr $(,)?,
    ) => {{
        $test_env.mount_execute_block(
            $mock_name.into(),
            ::serde_json::json!({
                "prevBlockHash": $parent,
                "transactions": $included_transactions,
            }),
            $crate::execute_block_response!(
                number: $number,
                hash: $hash,
                parent: $parent,
                included_transactions: $included_transactions
            )
        )
        .await
    }};
    (
        $test_env:ident,
        number: $number:expr,
        hash: $hash:expr,
        included_transactions: $included_transactions:expr,
        parent: $parent:expr $(,)?
    ) => {
        mount_executed_block!(
            $test_env,
            mock_name: None,
            number: $number,
            hash: $hash,
            parent: $parent,
            included_transactions: $included_transactions
        )
    };
}

pub struct TestExecutor {
    pub mock_grpc: MockGrpc,
}

impl TestExecutor {
    pub async fn mount_get_commitment_state(&self, commitment_state: CommitmentState) {
        astria_grpc_mock::Mock::for_rpc_given(
            "get_commitment_state",
            astria_grpc_mock::matcher::message_type::<GetCommitmentStateRequest>(),
        )
            .respond_with(astria_grpc_mock::response::constant_response(
                commitment_state,
            ))
            .expect(1..)
            .mount(&self.mock_grpc.mock_server)
            .await;
    }

    pub async fn mount_execute_block<S: serde::Serialize>(
        &self,
        mock_name: Option<&str>,
        _expected_pbjson: S,
        response: ExecuteBlockResponse,
    ) -> astria_grpc_mock::MockGuard {
        use astria_grpc_mock::{
            response::constant_response,
            Mock,
        };

        let mut mock = Mock::for_rpc_given("execute_block", AlwaysMatches {})
            .respond_with(constant_response(response));
        if let Some(name) = mock_name {
            mock = mock.with_name(name);
        }
        mock.expect(1)
            .mount_as_scoped(&self.mock_grpc.mock_server)
            .await
    }
}

// TODO - this is a hack to bypass request body matching. Fix this
struct AlwaysMatches();

impl Match for AlwaysMatches {
    fn matches(&self, _req: &Request<AnyMessage>) -> bool {
        true
    }
}
