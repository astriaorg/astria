use std::collections::{
    BTreeMap,
    HashMap,
};

use astria_core::{
    crypto::VerificationKey,
    generated::astria::signer::v1::{
        frost_participant_service_client::FrostParticipantServiceClient,
        CommitmentWithIdentifier,
        RoundOneRequest,
        RoundTwoRequest,
    },
    primitive::v1::Address,
    protocol::transaction::v1::{
        Transaction,
        TransactionBody,
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    WrapErr as _,
};
use frost_ed25519::{
    keys::PublicKeyPackage,
    round1::{
        self,
        SigningCommitments,
    },
    round2::SignatureShare,
    Identifier,
};
use futures::StreamExt as _;
use prost::{
    Message as _,
    Name as _,
};

pub(crate) struct Builder {
    pub(super) frost_min_signers: usize,
    pub(super) public_key_package: PublicKeyPackage,
    pub(super) sequencer_address_prefix: String,
    pub(super) participant_clients: Vec<FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl Builder {
    pub(super) fn try_build(self) -> eyre::Result<FrostSigner> {
        let Self {
            frost_min_signers: min_signers,
            public_key_package,
            sequencer_address_prefix,
            participant_clients,
        } = self;

        // XXX: VerifiyingKey<Ed25519Sha512>::serialize delegates to
        // SerializableElement<Ed25519Sha512>::serialize, which then
        // delegates to [`Ed25519ScalarField::serialize`], which just
        // yields a [u8; 32].
        //
        // [`VerifyingKey::serialize`] itself turns it into a vec.
        //
        // [`Ed25519ScalarField`]: https://docs.rs/frost-ed25519/2.1.0/src/frost_ed25519/lib.rs.html#68-70
        // [`VerifyingKey::serialize`]:  https://docs.rs/frost-core/2.1.0/src/frost_core/serialization.rs.html#22-26
        let verifying_key_bytes = public_key_package
            .verifying_key()
            .serialize()
            .wrap_err("failed to extract verifying key as raw bytes")?;
        let verifying_key: VerificationKey = VerificationKey::try_from(&*verifying_key_bytes)
            .wrap_err(
                "failed to construct ed25519 verification key from verification key extracted \
                 from frost public key package",
            )?;
        let address = Address::builder()
            .array(*verifying_key.address_bytes())
            .prefix(&sequencer_address_prefix)
            .try_build()
            .wrap_err_with(|| {
                format!(
                    "failed to build address given public key package and address prefix \
                     `{sequencer_address_prefix}`"
                )
            })?;

        ensure!(
            participant_clients.len() == min_signers,
            "not enough participant clients; need at least {min_signers}"
        );

        Ok(FrostSigner {
            min_signers,
            public_key_package,
            address,
            participant_clients,
            initialized_participant_clients: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FrostSigner {
    min_signers: usize,
    public_key_package: PublicKeyPackage,
    address: Address,
    participant_clients: Vec<FrostParticipantServiceClient<tonic::transport::Channel>>,
    initialized_participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl FrostSigner {
    pub(crate) async fn initialize_participant_clients(&mut self) -> eyre::Result<()> {
        use astria_core::generated::astria::signer::v1::GetVerifyingShareRequest;
        use frost_ed25519::keys::VerifyingShare;

        for client in &mut self.participant_clients {
            let resp = client
                .get_verifying_share(GetVerifyingShareRequest {})
                .await
                .wrap_err("failed to get verifying share")?;
            let verifying_share = VerifyingShare::deserialize(&resp.into_inner().verifying_share)
                .wrap_err("failed to deserialize verifying share")?;
            let identifier = self
                .public_key_package
                .verifying_shares()
                .iter()
                .find(|(_, vs)| vs == &&verifying_share)
                .map(|(id, _)| id)
                .ok_or_else(|| eyre!("failed to find identifier for verifying share"))?;
            self.initialized_participant_clients
                .insert(identifier.to_owned(), client.clone());
        }

        ensure!(
            self.initialized_participant_clients.len()
                == self.public_key_package.verifying_shares().len(),
            "failed to initialize all participant clients; are there duplicate endpoints?"
        );
        Ok(())
    }

    pub(super) async fn sign(&self, tx: TransactionBody) -> eyre::Result<Transaction> {
        // part 1: gather commitments from participants
        let round_1_results = self.execute_round_1().await;
        ensure!(
            round_1_results.responses.len() >= self.min_signers,
            "not enough part 1 responses received; want at least `{}`, got `{}`",
            self.min_signers,
            round_1_results.responses.len()
        );

        // part 2: gather signature shares from participants
        let tx_bytes = tx.to_raw().encode_to_vec();
        let sig_shares = self
            .execute_round_2(round_1_results.responses, tx_bytes.clone())
            .await;
        ensure!(
            sig_shares.len() >= self.min_signers,
            "not enough part 2 signature shares received; want at least `{}`, got `{}`",
            self.min_signers,
            sig_shares.len()
        );

        // finally, aggregate and create signature
        let signing_package = frost_ed25519::SigningPackage::new(
            round_1_results.signing_package_commitments,
            &tx_bytes,
        );
        let signature =
            frost_ed25519::aggregate(&signing_package, &sig_shares, &self.public_key_package)
                .wrap_err("failed to aggregate signature shares")?;

        let raw_transaction = astria_core::generated::astria::protocol::transaction::v1::Transaction {
                body: Some(pbjson_types::Any {
                    type_url: astria_core::generated::astria::protocol::transaction::v1::TransactionBody::type_url(),
                    value: tx_bytes.into(),
                }),
                signature: signature
                    .serialize()
                    .wrap_err("failed to serialize signature")?
                    .into(),
                public_key: self.public_key_package
                    .verifying_key()
                    .serialize()
                    .wrap_err("failed to serialize verifying key")?
                    .into(),
            };
        let transaction = Transaction::try_from_raw(raw_transaction)
            .wrap_err("failed to convert raw transaction to transaction")?;

        Ok(transaction)
    }

    pub(crate) fn address(&self) -> &Address {
        &self.address
    }
}

struct Round1Results {
    responses: Vec<Round1Response>,
    signing_package_commitments: BTreeMap<Identifier, round1::SigningCommitments>,
}

struct Round1Response {
    id: Identifier,
    commitment: axum::body::Bytes,
    request_identifier: u32,
}

impl FrostSigner {
    async fn execute_round_1(&self) -> Round1Results {
        let mut stream = futures::stream::FuturesUnordered::new();
        for (id, client) in &self.initialized_participant_clients {
            let client = client.clone();
            stream.push(execute_round_1(client, *id));
        }

        let mut responses = vec![];
        let mut signing_package_commitments: BTreeMap<Identifier, round1::SigningCommitments> =
            BTreeMap::new();
        while let Some(res) = stream.next().await {
            match res {
                Ok((id, commitment, request_identifier)) => {
                    signing_package_commitments.insert(id, commitment);
                    responses.push(Round1Response {
                        id,
                        commitment: commitment
                            .serialize()
                            .expect("commitment must be serializable, as we just deserialized it")
                            .into(),
                        request_identifier,
                    });
                }
                Err(e) => {
                    tracing::warn!("failed to get part 1 response: {e}");
                }
            }
        }

        Round1Results {
            responses,
            signing_package_commitments,
        }
    }

    async fn execute_round_2(
        &self,
        responses: Vec<Round1Response>,
        tx_bytes: Vec<u8>,
    ) -> BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> {
        let mut stream = futures::stream::FuturesUnordered::new();
        let request_commitments: Vec<CommitmentWithIdentifier> = responses
            .iter()
            .map(
                |Round1Response {
                     id,
                     commitment,
                     ..
                 }| CommitmentWithIdentifier {
                    commitment: commitment.clone(),
                    participant_identifier: id.serialize().into(),
                },
            )
            .collect();
        for Round1Response {
            id,
            request_identifier,
            ..
        } in responses
        {
            let client = self
                .initialized_participant_clients
                .get(&id)
                .expect(
                    "participant client must exist in mapping, as we received a commitment from \
                     them in part 1, meaning we already have their client",
                )
                .clone();
            let request_commitments = request_commitments.clone();
            let tx_bytes = tx_bytes.clone();
            stream.push(execute_round_2(
                client,
                id,
                request_identifier,
                tx_bytes,
                request_commitments,
            ));
        }

        let mut sig_shares = BTreeMap::new();
        while let Some(res) = stream.next().await {
            match res {
                Ok((id, sig_share)) => {
                    sig_shares.insert(id, sig_share);
                }
                Err(e) => {
                    tracing::warn!("failed to get part 2 response: {e}");
                }
            }
        }

        sig_shares
    }
}

async fn execute_round_1(
    mut client: FrostParticipantServiceClient<tonic::transport::Channel>,
    participant_id: Identifier,
) -> eyre::Result<(Identifier, SigningCommitments, u32)> {
    let resp = client
        .execute_round_one(RoundOneRequest {})
        .await
        .wrap_err_with(|| {
            format!("failed to get part 1 response for participant with id {participant_id:?}")
        })?
        .into_inner();
    let commitment =
        round1::SigningCommitments::deserialize(&resp.commitment).wrap_err_with(|| {
            format!(
                "failed to deserialize commitment from participant with identifier \
                 {participant_id:?}"
            )
        })?;
    Ok((participant_id, commitment, resp.request_identifier))
}

async fn execute_round_2(
    mut client: FrostParticipantServiceClient<tonic::transport::Channel>,
    participant_id: Identifier,
    request_identifier: u32,
    message: Vec<u8>,
    commitments: Vec<CommitmentWithIdentifier>,
) -> eyre::Result<(Identifier, SignatureShare)> {
    let resp = client
        .execute_round_two(RoundTwoRequest {
            request_identifier,
            message: message.into(),
            commitments,
        })
        .await
        .wrap_err_with(|| {
            format!("failed to get part 2 response for participant with id {participant_id:?}")
        })?
        .into_inner();
    let sig_share = frost_ed25519::round2::SignatureShare::deserialize(&resp.signature_share)
        .wrap_err_with(|| {
            format!(
                "failed to deserialize signature share from participant with identifier \
                 {participant_id:?}"
            )
        })?;
    Ok((participant_id, sig_share))
}
