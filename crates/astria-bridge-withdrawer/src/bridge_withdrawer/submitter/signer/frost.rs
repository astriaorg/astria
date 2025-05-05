use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    fmt::Display,
};

use astria_core::{
    crypto::VerificationKey,
    generated::astria::signer::v1::{
        frost_participant_service_client::FrostParticipantServiceClient,
        CommitmentWithIdentifier,
        ExecuteRoundOneRequest,
        ExecuteRoundTwoRequest,
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
    round1,
    round2::{
        self,
        SignatureShare,
    },
};
use futures::StreamExt as _;
use prost::{
    bytes::Bytes,
    Message as _,
    Name as _,
};

pub(super) struct Builder {
    pub(super) frost_min_signers: usize,
    pub(super) frost_participant_endpoints: crate::config::FrostParticipantEndpoints,
    pub(super) frost_public_key_package_path: String,
    pub(super) sequencer_address_prefix: String,
}

impl Builder {
    pub(super) fn try_build(self) -> eyre::Result<Frost> {
        fn read_frost_key<P: AsRef<std::path::Path>>(
            path: P,
        ) -> astria_eyre::eyre::Result<PublicKeyPackage> {
            let key_str = std::fs::read_to_string(path)
                .wrap_err("failed to read frost public key package")?;
            serde_json::from_str::<PublicKeyPackage>(&key_str)
                .wrap_err("failed to deserialize public key package")
        }

        let Self {
            frost_min_signers: min_signers,
            frost_participant_endpoints,
            frost_public_key_package_path,
            sequencer_address_prefix,
        } = self;

        ensure!(
            min_signers > 0,
            "minimum number of signers must be greater than 0"
        );

        ensure!(
            frost_participant_endpoints.len() >= min_signers,
            "not enough participant clients; need at least `{min_signers}`, but only `{}` were \
             provided",
            frost_participant_endpoints.len(),
        );

        let participant_clients: Vec<_> = frost_participant_endpoints
            .into_iter()
            .map(|endpoint| {
                FrostParticipantServiceClient::new(
                    tonic::transport::Endpoint::from(endpoint).connect_lazy(),
                )
            })
            .collect();

        let public_key_package =
            read_frost_key(&frost_public_key_package_path).wrap_err_with(|| {
                format!(
                    "failed reading frost public key package from file \
                     `{frost_public_key_package_path}`"
                )
            })?;

        // XXX: VerifiyingKey<Ed25519Sha512>::serialize delegates to
        // SerializableElement<Ed25519Sha512>::serialize, which then
        // delegates to [`Ed25519ScalarField::serialize`], which just
        // yields a [u8; 32].
        //
        // [`VerifyingKey::serialize`] itself turns it into a Vec<u8>.
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
        Ok(Frost {
            min_signers,
            public_key_package,
            address,
            participant_clients,
            initialized_participant_clients: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct Identifier {
    inner: frost_ed25519::Identifier,
    as_bytes: [u8; 32],
}

impl Identifier {
    fn new(inner: frost_ed25519::Identifier) -> Self {
        Self {
            inner,
            as_bytes: inner
                .serialize()
                .try_into()
                .expect("the frost ed25519 identifier must be a 32 bytes"),
        }
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.as_bytes
    }

    fn get(self) -> frost_ed25519::Identifier {
        self.inner
    }
}
impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("0x")?;
        for byte in self.as_bytes {
            f.write_fmt(format_args!("{byte:x}"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) struct Frost {
    min_signers: usize,
    public_key_package: PublicKeyPackage,
    address: Address,
    participant_clients: Vec<FrostParticipantServiceClient<tonic::transport::Channel>>,
    initialized_participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl Frost {
    pub(super) fn address(&self) -> &Address {
        &self.address
    }

    pub(super) async fn initialize_participant_clients(&mut self) -> eyre::Result<()> {
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
                .find_map(|(id, vs)| (vs == &verifying_share).then_some(Identifier::new(*id)))
                .ok_or_else(|| eyre!("failed to find identifier for verifying share"))?;
            self.initialized_participant_clients
                .insert(identifier, client.clone());
        }

        ensure!(
            self.initialized_participant_clients.len()
                == self.public_key_package.verifying_shares().len(),
            "failed to initialize all participant clients; are there duplicate endpoints?"
        );
        Ok(())
    }

    pub(super) async fn sign(
        &self,
        transaction_body: TransactionBody,
    ) -> eyre::Result<Transaction> {
        let round_one_results = self
            .execute_round_one()
            .await
            .wrap_err("round one failed")?;

        let encoded_transaction_body =
            prost::bytes::Bytes::from(transaction_body.to_raw().encode_to_vec());
        let sig_shares = self
            .execute_round_two(&round_one_results, encoded_transaction_body.clone())
            .await
            .wrap_err("round two failed")?;

        let transaction = self
            .aggregate_transaction(encoded_transaction_body, round_one_results, &sig_shares)
            .wrap_err(
                "failed aggregating transaction body and the results of the round 1 and 2 \
                 threshold scheme into a signed Astria transaction",
            )?;

        Ok(transaction)
    }

    async fn execute_round_one(&self) -> eyre::Result<Vec<RoundOneResult>> {
        let mut stream = futures::stream::FuturesUnordered::new();
        for (id, client) in &self.initialized_participant_clients {
            let client = client.clone();
            stream.push(async move {
                execute_round_one(client, *id).await.wrap_err_with(|| {
                    format!("failed executing round one step for participant client with ID `{id}`")
                })
            });
        }

        let mut responses = vec![];
        while let Some(res) = stream.next().await {
            match res {
                Ok(res) => {
                    responses.push(res);
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to get part 1 response for one of the threshold participants; dropping its response and continuing with the others");
                }
            }
        }

        ensure!(
            responses.len() >= self.min_signers,
            "not enough part 1 responses received; want at least `{}`, got `{}`",
            self.min_signers,
            responses.len()
        );
        Ok(responses)
    }

    async fn execute_round_two(
        &self,
        responses: &[RoundOneResult],
        tx_bytes: prost::bytes::Bytes,
    ) -> eyre::Result<BTreeMap<frost_ed25519::Identifier, round2::SignatureShare>> {
        let mut stream = futures::stream::FuturesUnordered::new();
        let request_commitments: Vec<CommitmentWithIdentifier> = responses
            .iter()
            .map(
                |RoundOneResult {
                     participant_identifier,
                     raw_signing_commitments,
                     ..
                 }| CommitmentWithIdentifier {
                    commitment: raw_signing_commitments.clone(),
                    participant_identifier: Bytes::copy_from_slice(
                        participant_identifier.as_bytes(),
                    ),
                },
            )
            .collect();

        for RoundOneResult {
            participant_identifier,
            request_identifier,
            ..
        } in responses
        {
            let client = self
                .initialized_participant_clients
                .get(participant_identifier)
                .expect(
                    "participant client must exist in mapping, as we received a commitment from \
                     them in part 1, meaning we already have their client",
                )
                .clone();
            let request_commitments = request_commitments.clone();
            let tx_bytes = tx_bytes.clone();
            stream.push(async move {
                execute_round_two(
                    client,
                    *participant_identifier,
                    *request_identifier,
                    tx_bytes,
                    request_commitments,
                )
                .await
                .wrap_err_with(|| {
                    format!(
                        "failed executing round two step for participant client with ID \
                         `{participant_identifier}`"
                    )
                })
            });
        }

        let mut sig_shares = BTreeMap::new();
        while let Some(res) = stream.next().await {
            match res {
                Ok((participant_identifier, sig_share)) => {
                    sig_shares.insert(participant_identifier.get(), sig_share);
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to get part 2 response for one of the threshold particpiants; dropping it and continuing with the rest");
                }
            }
        }

        ensure!(
            sig_shares.len() >= self.min_signers,
            "not enough part 2 signature shares received; want at least `{}`, got `{}`",
            self.min_signers,
            sig_shares.len()
        );
        Ok(sig_shares)
    }

    fn aggregate_transaction(
        &self,
        encoded_transaction_body: prost::bytes::Bytes,
        round_one_results: Vec<RoundOneResult>,
        sig_shares: &BTreeMap<frost_ed25519::Identifier, round2::SignatureShare>,
    ) -> eyre::Result<Transaction> {
        let signing_commitments = round_one_results
            .into_iter()
            .map(|res| {
                (
                    res.participant_identifier.get(),
                    res.decoded_signing_commitments,
                )
            })
            .collect();
        let signing_package =
            frost_ed25519::SigningPackage::new(signing_commitments, &encoded_transaction_body);
        let signature =
            frost_ed25519::aggregate(&signing_package, sig_shares, &self.public_key_package)
                .wrap_err("failed to aggregate signature shares")?;

        let raw_transaction = astria_core::generated::astria::protocol::transaction::v1::Transaction {
                body: Some(pbjson_types::Any {
                    type_url: astria_core::generated::astria::protocol::transaction::v1::TransactionBody::type_url(),
                    value: encoded_transaction_body,
                }),
                signature: signature
                    .serialize()
                    .wrap_err("failed to serialize aggregated threshold signature")?
                    .into(),
                public_key: self.public_key_package
                    .verifying_key()
                    .serialize()
                    .wrap_err("failed to serialize verifying key of public key package")?
                    .into(),
            };
        let transaction = Transaction::try_from_raw(raw_transaction)
            .wrap_err("failed to convert raw transaction to transaction")?;
        Ok(transaction)
    }
}

async fn execute_round_one(
    mut client: FrostParticipantServiceClient<tonic::transport::Channel>,
    participant_identifier: Identifier,
) -> eyre::Result<RoundOneResult> {
    let resp = client
        .execute_round_one(ExecuteRoundOneRequest {})
        .await
        .wrap_err("ExecuteRoundOne RPC failed")?
        .into_inner();
    let decoded_signing_commitments = round1::SigningCommitments::deserialize(&resp.commitment)
        .wrap_err_with(|| {
            "failed deserializing round 1 signing commitments from `.commitment` field of RPC \
             response"
        })?;
    Ok(RoundOneResult {
        participant_identifier,
        raw_signing_commitments: resp.commitment,
        decoded_signing_commitments,
        request_identifier: resp.request_identifier,
    })
}

async fn execute_round_two(
    mut client: FrostParticipantServiceClient<tonic::transport::Channel>,
    participant_id: Identifier,
    request_identifier: u32,
    message: prost::bytes::Bytes,
    commitments: Vec<CommitmentWithIdentifier>,
) -> eyre::Result<(Identifier, SignatureShare)> {
    let resp = client
        .execute_round_two(ExecuteRoundTwoRequest {
            request_identifier,
            message,
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

/// The result of executing the round 1 step of the frost signing scheme for
/// a single participant.
struct RoundOneResult {
    participant_identifier: Identifier,
    decoded_signing_commitments: round1::SigningCommitments,
    raw_signing_commitments: prost::bytes::Bytes,
    request_identifier: u32,
}
