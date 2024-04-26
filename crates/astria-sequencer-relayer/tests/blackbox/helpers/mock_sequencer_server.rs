use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::{
    generated::sequencerblock::v1alpha1::{
        sequencer_service_server::{
            SequencerService,
            SequencerServiceServer,
        },
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        GetFilteredSequencerBlockRequest,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    primitive::v1::RollupId,
    protocol::test_utils::ConfigureSequencerBlock,
    sequencerblock::v1alpha1::SequencerBlock,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use astria_grpc_mock::{
    matcher::message_type,
    response::constant_response,
    Mock,
    MockServer,
};
use tendermint::account::Id as AccountId;
use tokio::task::JoinHandle;
use tonic::{
    transport::Server,
    Request,
    Response,
    Status,
};

const GET_SEQUENCER_BLOCK_GRPC_NAME: &str = "get_sequencer_block";
const GET_FILTERED_SEQUENCER_BLOCK_GRPC_NAME: &str = "get_filtered_sequencer_block";

pub struct MockSequencerServer {
    pub _server: JoinHandle<eyre::Result<()>>,
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

    pub async fn mount_sequencer_block_response<const RELAY_SELF: bool>(
        &self,
        account: AccountId,
        block_to_mount: SequencerBlockToMount,
        debug_name: impl Into<String>,
    ) -> astria_grpc_mock::MockGuard {
        let proposer = if RELAY_SELF {
            account
        } else {
            AccountId::try_from(vec![0u8; 20]).unwrap()
        };

        let should_corrupt = matches!(block_to_mount, SequencerBlockToMount::BadAtHeight(_));

        let block = match block_to_mount {
            SequencerBlockToMount::GoodAtHeight(height)
            | SequencerBlockToMount::BadAtHeight(height) => ConfigureSequencerBlock {
                block_hash: Some([99u8; 32]),
                height,
                proposer_address: Some(proposer),
                sequence_data: vec![(
                    RollupId::from_unhashed_bytes(b"some_rollup_id"),
                    vec![99u8; 32],
                )],
                ..Default::default()
            }
            .make(),
            SequencerBlockToMount::Block(block) => block,
        };

        let mut block = block.into_raw();
        if should_corrupt {
            let header = block.header.as_mut().unwrap();
            header.data_hash = [0; 32].to_vec();
        }

        Mock::for_rpc_given(
            GET_SEQUENCER_BLOCK_GRPC_NAME,
            message_type::<GetSequencerBlockRequest>(),
        )
        .respond_with(constant_response(block))
        .up_to_n_times(1)
        .expect(1)
        .with_name(debug_name)
        .mount_as_scoped(&self.mock_server)
        .await
    }
}

// allow: this is not performance-critical, with likely only one instance per test fixture.
#[allow(clippy::large_enum_variant)]
pub enum SequencerBlockToMount {
    GoodAtHeight(u32),
    BadAtHeight(u32),
    Block(SequencerBlock),
}

struct SequencerServiceImpl(MockServer);

#[tonic::async_trait]
impl SequencerService for SequencerServiceImpl {
    async fn get_sequencer_block(
        self: Arc<Self>,
        request: Request<GetSequencerBlockRequest>,
    ) -> Result<Response<RawSequencerBlock>, Status> {
        self.0
            .handle_request(GET_SEQUENCER_BLOCK_GRPC_NAME, request)
            .await
    }

    async fn get_filtered_sequencer_block(
        self: Arc<Self>,
        request: Request<GetFilteredSequencerBlockRequest>,
    ) -> Result<Response<RawFilteredSequencerBlock>, Status> {
        self.0
            .handle_request(GET_FILTERED_SEQUENCER_BLOCK_GRPC_NAME, request)
            .await
    }
}
