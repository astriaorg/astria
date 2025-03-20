use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::generated::astria::signer::v1::{
    frost_participant_service_server::FrostParticipantService,
    GetVerifyingShareRequest,
    RoundOneRequest,
    RoundOneResponse,
    RoundTwoRequest,
    RoundTwoResponse,
    VerifyingShare,
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
use tracing::{
    debug,
    instrument,
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
        self.next_request_id = self.next_request_id.saturating_add(1);
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
    /// Creates a new `Server` instance.
    ///
    /// # Errors
    ///
    /// - If the secret key package file cannot be read.
    /// - If the secret key package cannot be deserialized.
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
    #[instrument(skip_all)]
    async fn get_verifying_share(
        self: Arc<Self>,
        _request: Request<GetVerifyingShareRequest>,
    ) -> Result<Response<VerifyingShare>, Status> {
        let verifying_share = self
            .secret_package
            .verifying_share()
            .to_owned()
            .serialize()
            .map_err(|e| Status::internal(format!("failed to serialize verifying share: {e}")))?
            .into();
        Ok(Response::new(VerifyingShare {
            verifying_share,
        }))
    }

    #[instrument(skip_all)]
    async fn execute_round_one(
        self: Arc<Self>,
        _request: Request<RoundOneRequest>,
    ) -> Result<Response<RoundOneResponse>, Status> {
        self.metrics.increment_part_1_request_count();
        let mut rng = OsRng;
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
        debug!(request_identifier, "generated part 1 response");
        Ok(Response::new(RoundOneResponse {
            request_identifier,
            commitment,
        }))
    }

    #[instrument(skip_all)]
    async fn execute_round_two(
        self: Arc<Self>,
        request: Request<RoundTwoRequest>,
    ) -> Result<Response<RoundTwoResponse>, Status> {
        self.metrics.increment_part_2_request_count();
        let request = request.into_inner();
        let mut state = self.state.lock().await;
        let Some(nonce) = state
            .request_id_to_nonces
            .remove(&request.request_identifier)
        else {
            return Err(Status::invalid_argument("invalid request identifier"));
        };

        if let Err(e) = self.verifier.verify_message_to_sign(&request.message).await {
            self.metrics.increment_invalid_message_count();
            return Err(Status::invalid_argument(format!(
                "signing message is invalid: {e}"
            )));
        };

        self.metrics.increment_valid_message_count();

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
        debug!(request.request_identifier, "generated part 2 response");

        Ok(Response::new(RoundTwoResponse {
            signature_share,
        }))
    }
}
