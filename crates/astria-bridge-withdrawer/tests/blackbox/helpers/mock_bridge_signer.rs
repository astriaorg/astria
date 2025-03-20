use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::{
    self,
    generated::astria::signer::v1::{
        frost_participant_service_server::{
            FrostParticipantService,
            FrostParticipantServiceServer,
        },
        ExecuteRoundOneRequest,
        ExecuteRoundTwoRequest,
        GetVerifyingShareRequest,
        RoundOneResponse,
        RoundTwoResponse,
        VerifyingShare,
    },
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
use frost_ed25519::round1;
use rand::SeedableRng as _;
use tokio::task::JoinHandle;
use tonic::{
    transport::Server,
    Request,
    Response,
    Status,
};

#[expect(
    clippy::module_name_repetitions,
    reason = "naming is helpful for clarity here"
)]
pub struct MockBridgeSignerServer {
    _server: JoinHandle<eyre::Result<()>>,
    pub(crate) mock_server: MockServer,
    pub(crate) local_addr: SocketAddr,
    secret_package: frost_ed25519::keys::KeyPackage,
}

impl MockBridgeSignerServer {
    pub(crate) async fn spawn(secret_package: frost_ed25519::keys::KeyPackage) -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let mock_server = MockServer::new();

        let server = {
            let sequencer_service = FrostParticipantServiceImpl {
                server: mock_server.clone(),
                secret_package: secret_package.clone(),
                nonce: tokio::sync::Mutex::new(None),
            };
            tokio::spawn(async move {
                Server::builder()
                    .add_service(FrostParticipantServiceServer::new(sequencer_service))
                    .serve_with_incoming(TcpListenerStream::new(listener))
                    .await
                    .wrap_err("gRPC sequencer server failed")
            })
        };
        Self {
            _server: server,
            mock_server,
            local_addr,
            secret_package,
        }
    }

    pub(crate) async fn mount_get_verifying_share_response(&self, debug_name: impl Into<String>) {
        let resp = VerifyingShare {
            verifying_share: self
                .secret_package
                .verifying_share()
                .to_owned()
                .serialize()
                .expect("can serialize verifying share")
                .into(),
        };
        Mock::for_rpc_given(
            "get_verifying_share",
            message_type::<GetVerifyingShareRequest>(),
        )
        .respond_with(constant_response(resp))
        .up_to_n_times(1)
        .expect(1)
        .with_name(debug_name)
        .mount(&self.mock_server)
        .await;
    }
}

struct FrostParticipantServiceImpl {
    server: MockServer,
    secret_package: frost_ed25519::keys::KeyPackage,
    nonce: tokio::sync::Mutex<Option<round1::SigningNonces>>,
}

#[tonic::async_trait]
impl FrostParticipantService for FrostParticipantServiceImpl {
    async fn get_verifying_share(
        self: Arc<Self>,
        request: Request<GetVerifyingShareRequest>,
    ) -> Result<Response<VerifyingShare>, Status> {
        self.server
            .handle_request("get_verifying_share", request)
            .await
    }

    async fn execute_round_one(
        self: Arc<Self>,
        _request: Request<ExecuteRoundOneRequest>,
    ) -> Result<Response<RoundOneResponse>, Status> {
        let mut rng = rand_chacha::ChaChaRng::seed_from_u64(0);
        let (nonces, commitments) =
            frost_ed25519::round1::commit(self.secret_package.signing_share(), &mut rng);
        let commitment = commitments
            .serialize()
            .map_err(|e| Status::internal(format!("failed to serialize commitments: {e}")))?
            .into();
        let mut nonce = self.nonce.lock().await;
        *nonce = Some(nonces.clone());
        Ok(Response::new(RoundOneResponse {
            request_identifier: 0,
            commitment,
        }))
    }

    async fn execute_round_two(
        self: Arc<Self>,
        request: Request<ExecuteRoundTwoRequest>,
    ) -> Result<Response<RoundTwoResponse>, Status> {
        let request = request.into_inner();
        let signing_commitments = request
            .commitments
            .into_iter()
            .filter_map(|c| {
                Some((
                    frost_ed25519::Identifier::deserialize(&c.participant_identifier).ok()?,
                    round1::SigningCommitments::deserialize(&c.commitment).ok()?,
                ))
            })
            .collect();
        let signing_package =
            frost_ed25519::SigningPackage::new(signing_commitments, &request.message);

        let nonce = {
            let mut nonce = self.nonce.lock().await;
            nonce
                .take()
                .ok_or_else(|| Status::internal("nonce not set"))?
        };
        let signature_share =
            frost_ed25519::round2::sign(&signing_package, &nonce, &self.secret_package)
                .map_err(|e| Status::internal(format!("failed to sign: {e}")))?
                .serialize()
                .into();

        Ok(Response::new(RoundTwoResponse {
            signature_share,
        }))
    }
}
