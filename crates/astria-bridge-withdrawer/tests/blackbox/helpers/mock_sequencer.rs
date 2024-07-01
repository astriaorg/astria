use astria_bridge_withdrawer::bridge_withdrawer;
use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    generated::{
        protocol::transaction::v1alpha1::{
            IbcHeight,
            SignedTransaction,
        },
        sequencerblock::v1alpha1::{
            GetPendingNonceRequest,
            GetPendingNonceResponse,
        },
    },
    primitive::v1::asset::default_native_asset,
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use astria_grpc_mock::{
    Mock,
    MockGuard,
    MockServer,
};
use tendermint_rpc::{
    endpoint::broadcast::tx_sync,
    request,
};
use tracing::debug;

const GET_PENDING_NONCE_GRPC_NAME: &str = "get_pending_nonce";

pub struct MockSequencerServer {
    _server: JoinHandle<eyre::Result<()>>,
    pub mock_server: MockServer,
    pub local_addr: SocketAddr,
}

impl MockSequencerServer {
    pub async fn spawn() -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let mock_server = MockServer::new();

        let server = {
            let sequencer_service = SequencerServiceImpl(mock_server.clone());
            tokio::spawn(async move {
                Server::builder()
                    .add_service(SequencerServiceServer::new(sequencer_service))
                    .serve_with_incoming(TcpListenerStream::new(listener))
                    .await
                    .wrap_err("gRPC sequencer server failed")
            })
        };
        Self {
            _server: server,
            mock_server,
            local_addr,
        }
    }

    pub async fn mount_pending_nonce_response(
        &self,
        nonce_to_mount: u32,
        debug_name: impl Into<String>,
    ) {
        let nonce_req = GetPendingNonceResponse {
            inner: nonce_to_mount,
        };
        Mock::for_rpc_given(
            GET_PENDING_NONCE_GRPC_NAME,
            message_type::<GetPendingNonceRequest>(),
        )
        .respond_with(constant_response(nonce_req))
        .up_to_n_times(1)
        .expect(1)
        .with_name(debug_name)
        .mount(&self.mock_server)
        .await;
    }

    pub async fn mount_pending_nonce_response_as_scoped(
        &self,
        nonce_to_mount: u32,
        debug_name: impl Into<String>,
    ) -> MockGuard {
        let nonce_req = GetPendingNonceResponse {
            inner: nonce_to_mount,
        };
        Mock::for_rpc_given(
            GET_PENDING_NONCE_GRPC_NAME,
            message_type::<GetPendingNonceRequest>(),
        )
        .respond_with(constant_response(nonce_req))
        .up_to_n_times(1)
        .expect(1)
        .with_name(debug_name)
        .mount_as_scoped(&self.mock_server)
        .await
    }
}

struct SequencerServiceImpl(MockServer);

#[tonic::async_trait]
impl SequencerService for SequencerServiceImpl {
    async fn get_sequencer_block(
        self: Arc<Self>,
        request: Request<GetSequencerBlockRequest>,
    ) -> Result<Response<RawSequencerBlock>, Status> {
        unimplemented!()
    }

    async fn get_filtered_sequencer_block(
        self: Arc<Self>,
        request: Request<GetFilteredSequencerBlockRequest>,
    ) -> Result<Response<RawFilteredSequencerBlock>, Status> {
        unimplemented!()
    }

    async fn get_pending_nonce(
        self: Arc<Self>,
        _request: Request<GetPendingNonceRequest>,
    ) -> Result<Response<GetPendingNonceResponse>, Status> {
        self.0
            .handle_request(GET_PENDING_NONCE_GRPC_NAME, request)
            .await
    }
}
