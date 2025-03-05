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
    round1,
    Identifier,
};
use futures::StreamExt as _;
use prost::{
    Message as _,
    Name as _,
};

pub(crate) struct FrostSignerBuilder {
    min_signers: Option<usize>,
    public_key_package: Option<PublicKeyPackage>,
    address_prefix: Option<String>,
    participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl FrostSignerBuilder {
    pub(crate) fn new() -> Self {
        Self {
            min_signers: None,
            public_key_package: None,
            address_prefix: None,
            participant_clients: HashMap::new(),
        }
    }

    pub(crate) fn min_signers(self, min_signers: usize) -> Self {
        Self {
            min_signers: Some(min_signers),
            ..self
        }
    }

    pub(crate) fn public_key_package(self, public_key_package: PublicKeyPackage) -> Self {
        Self {
            public_key_package: Some(public_key_package),
            ..self
        }
    }

    pub(crate) fn address_prefix(self, address_prefix: String) -> Self {
        Self {
            address_prefix: Some(address_prefix),
            ..self
        }
    }

    pub(crate) fn participant_clients(
        self,
        participant_clients: HashMap<
            Identifier,
            FrostParticipantServiceClient<tonic::transport::Channel>,
        >,
    ) -> Self {
        Self {
            participant_clients,
            ..self
        }
    }

    pub(crate) fn try_build(self) -> eyre::Result<FrostSigner> {
        let min_signers = self
            .min_signers
            .ok_or_else(|| eyre!("minimum number of signers is required"))?;
        let public_key_package = self
            .public_key_package
            .ok_or_else(|| eyre!("public key package is required"))?;
        let verifying_key_bytes: [u8; 32] = public_key_package
            .verifying_key()
            .serialize()
            .wrap_err("failed to serialize verifying key")?
            .try_into()
            .map_err(|_| eyre!("failed to convert verifying key to 32 bytes"))?;
        let verifying_key: VerificationKey = VerificationKey::try_from(verifying_key_bytes)
            .wrap_err("failed to build verification key")?;
        let address = Address::builder()
            .array(*verifying_key.address_bytes())
            .prefix(
                self.address_prefix
                    .ok_or_else(|| eyre!("astria address prefix is required"))?,
            )
            .try_build()
            .wrap_err("failed to build address")?;

        ensure!(
            self.participant_clients.len() == min_signers,
            "not enough participant clients; need at least {min_signers}"
        );

        Ok(FrostSigner {
            min_signers,
            public_key_package,
            address,
            participant_clients: self.participant_clients,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FrostSigner {
    min_signers: usize,
    public_key_package: PublicKeyPackage,
    address: Address,
    participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl FrostSigner {
    pub(crate) async fn sign(&self, tx: TransactionBody) -> eyre::Result<Transaction> {
        // part 1: gather commitments from participants
        let round_1_results = self.execute_round_1().await;
        ensure!(
            round_1_results.responses.len() >= self.min_signers,
            "not enough part 1 responses received; want at least {}, got {}",
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
            "not enough part 2 signature shares received; want at least {}, got {}",
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
        let stream = futures::stream::FuturesUnordered::new();
        for (id, client) in &self.participant_clients {
            let mut client = client.clone();
            stream.push(async move {
                let resp = client
                    .execute_round_one(RoundOneRequest {})
                    .await
                    .wrap_err(format!(
                        "failed to get part 1 response for participant with id {id:?}"
                    ))?;
                Ok((id, resp.into_inner()))
            });
        }
        let results: Vec<eyre::Result<_>> = stream.collect::<Vec<_>>().await;
        let mut signing_package_commitments: BTreeMap<Identifier, round1::SigningCommitments> =
            BTreeMap::new();

        let responses = results
            .into_iter()
            .filter_map(|res| match res {
                Ok((id, part1)) => {
                    let signing_commitment =
                        round1::SigningCommitments::deserialize(&part1.commitment).ok()?;
                    signing_package_commitments.insert(*id, signing_commitment);
                    Some(Round1Response {
                        id: *id,
                        commitment: part1.commitment,
                        request_identifier: part1.request_identifier,
                    })
                }
                Err(_) => None,
            })
            .collect::<Vec<_>>();
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
        let stream = futures::stream::FuturesUnordered::new();
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
            let mut client = self
                .participant_clients
                .get(&id)
                .expect(
                    "participant client must exist in mapping, as we received a commitment from \
                     them in part 1, meaning we already have their client",
                )
                .clone();
            let request_commitments = request_commitments.clone();
            let tx_bytes = tx_bytes.clone();
            stream.push(async move {
                let resp = client
                    .execute_round_two(RoundTwoRequest {
                        request_identifier,
                        message: tx_bytes.into(),
                        commitments: request_commitments,
                    })
                    .await
                    .wrap_err(format!(
                        "failed to get part 2 response for participant with id {id:?}"
                    ))?;
                Ok((id, resp.into_inner()))
            });
        }
        let results: Vec<eyre::Result<_>> = stream.collect::<Vec<_>>().await;
        let sig_shares: BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> = results
            .into_iter()
            .filter_map(|res| match res {
                Ok((id, part2)) => {
                    let sig_share =
                        frost_ed25519::round2::SignatureShare::deserialize(&part2.signature_share)
                            .ok()?;
                    Some((id, sig_share))
                }
                Err(_) => None,
            })
            .collect();

        sig_shares
    }
}
