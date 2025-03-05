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
        GetVerifyingShareRequest,
        GetVerifyingShareResponse,
        RoundOneRequest,
        RoundOneResponse,
        RoundTwoRequest,
        RoundTwoResponse,
    },
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::round1;
use rand::rngs::OsRng;
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
    pub(crate) local_addr: SocketAddr,
}

impl MockBridgeSignerServer {
    pub(crate) async fn spawn(secret_package: frost_ed25519::keys::KeyPackage) -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let server = {
            let sequencer_service = FrostParticipantServiceImpl {
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
            local_addr,
        }
    }
}

struct FrostParticipantServiceImpl {
    secret_package: frost_ed25519::keys::KeyPackage,
    nonce: tokio::sync::Mutex<Option<round1::SigningNonces>>,
}

#[tonic::async_trait]
impl FrostParticipantService for FrostParticipantServiceImpl {
    async fn get_verifying_share(
        self: Arc<Self>,
        _request: Request<GetVerifyingShareRequest>,
    ) -> Result<Response<GetVerifyingShareResponse>, Status> {
        let resp = GetVerifyingShareResponse {
            verifying_share: self
                .secret_package
                .verifying_share()
                .to_owned()
                .serialize()
                .expect("can serialize verifying share")
                .into(),
        };
        Ok(Response::new(resp))
    }

    async fn execute_round_one(
        self: Arc<Self>,
        _request: Request<RoundOneRequest>,
    ) -> Result<Response<RoundOneResponse>, Status> {
        let mut rng = OsRng;
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
        request: Request<RoundTwoRequest>,
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
