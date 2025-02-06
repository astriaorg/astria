use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::generated::astria::signer::v1::{
    frost_participant_service_server::FrostParticipantService,
    GetVerifyingShareRequest,
    GetVerifyingShareResponse,
    Part1Request,
    Part1Response,
    Part2Request,
    Part2Response,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use frost_ed25519::round1;
use rand::rngs::OsRng;
use tokio::sync::Mutex;
use tonic::{
    async_trait,
    Request,
    Response,
    Status,
};

use crate::{
    metrics::Metrics,
    Verifier,
};

struct State {
    next_request_id: u32,
    request_id_to_nonces: HashMap<u32, frost_ed25519::round1::SigningNonces>,
}

impl State {
    fn get_and_increment_next_request_id(&mut self) -> u32 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        request_id
    }
}

pub struct Server {
    verifier: Verifier,
    metrics: &'static Metrics,
    secret_package: frost_ed25519::keys::KeyPackage,
    state: Mutex<State>,
}

impl Server {
    pub fn new(
        secret_key_package_path: String,
        verifier: Verifier,
        metrics: &'static Metrics,
    ) -> eyre::Result<Self> {
        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )
        .wrap_err("failed to deserialize secret key package")?;

        Ok(Self {
            verifier,
            metrics,
            secret_package,
            state: Mutex::new(State {
                next_request_id: 0,
                request_id_to_nonces: HashMap::new(),
            }),
        })
    }
}

#[async_trait]
impl FrostParticipantService for Server {
    async fn get_verifying_share(
        self: Arc<Self>,
        _request: Request<GetVerifyingShareRequest>,
    ) -> Result<Response<GetVerifyingShareResponse>, Status> {
        let verifying_share = self
            .secret_package
            .verifying_share()
            .to_owned()
            .serialize()
            .map_err(|e| Status::internal(format!("failed to serialize verifying share: {e}")))?
            .into();
        Ok(Response::new(GetVerifyingShareResponse {
            verifying_share,
        }))
    }

    async fn part1(
        self: Arc<Self>,
        _request: Request<Part1Request>,
    ) -> Result<Response<Part1Response>, Status> {
        let mut rng = OsRng::default();
        let (nonces, commitments) =
            frost_ed25519::round1::commit(self.secret_package.signing_share(), &mut rng);
        let commitment = commitments
            .serialize()
            .map_err(|e| Status::internal(format!("failed to serialize commitments: {e}")))?
            .into();

        let mut state = self.state.lock().await;
        let request_identifier = state.get_and_increment_next_request_id();
        state
            .request_id_to_nonces
            .insert(request_identifier, nonces);
        Ok(Response::new(Part1Response {
            request_identifier,
            commitment,
        }))
    }

    async fn part2(
        self: Arc<Self>,
        request: Request<Part2Request>,
    ) -> Result<Response<Part2Response>, Status> {
        let request = request.into_inner();
        let mut state = self.state.lock().await;
        let Some(nonce) = state
            .request_id_to_nonces
            .remove(&request.request_identifier)
        else {
            return Err(Status::invalid_argument("invalid request identifier"));
        };

        // TODO: verify message
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

        let signature_share =
            frost_ed25519::round2::sign(&signing_package, &nonce, &self.secret_package)
                .map_err(|e| Status::internal(format!("failed to sign: {e}")))?
                .serialize()
                .into();

        Ok(Response::new(Part2Response {
            signature_share,
        }))
    }
}
