use std::collections::{
    BTreeMap,
    HashMap,
};

use astria_core::{
    crypto::VerificationKey,
    generated::astria::{
        protocol::transaction,
        signer::v1::{
            frost_participant_service_client::FrostParticipantServiceClient,
            CommitmentWithIdentifier,
            GetVerifyingShareRequest,
            Part1Request,
            Part2Request,
        },
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
    Context,
};
use ethers::types::Sign;
use frost_ed25519::{
    keys::{
        KeyPackage,
        PublicKeyPackage,
        VerifyingShare,
    },
    round1,
    Identifier,
};

use super::Signer;

pub(crate) async fn initialize_frost_participant_clients(
    endpoints: Vec<String>,
    public_key_package: PublicKeyPackage,
) -> eyre::Result<HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>> {
    // TODO: maybe remove this check, and just check we have >= min_signers in build()
    ensure!(
        endpoints.len() == public_key_package.verifying_shares().len(),
        "number of endpoints does not match number of participants"
    );

    let mut participant_clients = HashMap::new();
    for endpoint in endpoints {
        let mut client = FrostParticipantServiceClient::connect(endpoint)
            .await
            .wrap_err("failed to connect to participant")?;
        let resp = client
            .get_verifying_share(GetVerifyingShareRequest {})
            .await
            .wrap_err("failed to get verifying share")?;
        let verifying_share = VerifyingShare::deserialize(&resp.into_inner().verifying_share)
            .wrap_err("failed to deserialize verifying share")?;
        let identifier = public_key_package
            .verifying_shares()
            .iter()
            .find(|(_, vs)| vs == &&verifying_share)
            .map(|(id, _)| id)
            .ok_or_else(|| eyre!("failed to find identifier for verifying share"))?;
        participant_clients.insert(identifier.to_owned(), client);
    }

    ensure!(
        participant_clients.len() == public_key_package.verifying_shares().len(),
        "failed to initialize all participant clients; are there duplicate endpoints?"
    );

    Ok(participant_clients)
}

pub(crate) struct FrostSignerBuilder {
    key_package: Option<KeyPackage>,
    public_key_package: Option<PublicKeyPackage>,
    address_prefix: Option<String>,
    participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

impl FrostSignerBuilder {
    pub(crate) fn key_package(self, key_package: KeyPackage) -> Self {
        Self {
            key_package: Some(key_package),
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
        let key_package = self
            .key_package
            .ok_or_else(|| eyre!("key package is required"))?;
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

        Ok(FrostSigner {
            key_package,
            public_key_package,
            address,
            participant_clients: self.participant_clients,
        })
    }
}

pub(crate) struct FrostSigner {
    key_package: KeyPackage,
    public_key_package: PublicKeyPackage,
    address: Address,
    participant_clients:
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
}

#[tonic::async_trait]
impl Signer for FrostSigner {
    async fn sign(&self, tx: TransactionBody) -> eyre::Result<Transaction> {
        use futures::StreamExt as _;
        use prost::{
            Message as _,
            Name as _,
        };

        // TODO: -1 for min_other_signers; or do we want all participants
        // to be separate processes?
        let min_signers: usize = (*self.key_package.min_signers()).into();

        // part 1
        let stream = futures::stream::FuturesUnordered::new();
        for (id, client) in &self.participant_clients {
            let mut client = client.clone();
            stream.push(async move {
                let resp = client.part1(Part1Request {}).await.wrap_err(format!(
                    "failed to get part 1 commitment for participant with id {:?}",
                    id
                ))?;
                Ok((id, resp.into_inner()))
            });
        }
        let results: Vec<eyre::Result<_>> = stream.collect::<Vec<_>>().await;
        let mut commitments = Vec::new();
        let mut signing_package_commitments: BTreeMap<Identifier, round1::SigningCommitments> =
            BTreeMap::new();

        for res in results {
            if let Ok((id, part1)) = res {
                let signing_commitment = round1::SigningCommitments::deserialize(&part1.commitment)
                    .wrap_err("failed to deserialize signing commitment")?;
                signing_package_commitments.insert(*id, signing_commitment);
                commitments.push((id, part1.commitment, part1.request_identifier));
            };
        }
        ensure!(
            commitments.len() >= min_signers,
            "not enough part 1 commitments"
        );

        // part 2
        let stream = futures::stream::FuturesUnordered::new();
        let request_commitments: Vec<CommitmentWithIdentifier> = commitments
            .iter()
            .map(|(id, commitment, _)| CommitmentWithIdentifier {
                commitment: commitment.clone(),
                participant_identifier: id.serialize().to_vec().into(),
            })
            .collect();
        let tx_bytes = tx.to_raw().encode_to_vec();
        for (id, _, request_identifier) in commitments {
            let mut client = self
                .participant_clients
                .get(&id)
                .ok_or_else(|| eyre!("failed to find participant client"))?
                .clone();
            let request_commitments = request_commitments.clone();
            let tx = tx.clone();
            stream.push(async move {
                let resp = client
                    .part2(Part2Request {
                        request_identifier,
                        transaction_body: Some(tx.into_raw()), /* TODO: this needs to
                                                                * be bytes for
                                                                * determinism */
                        commitments: request_commitments,
                    })
                    .await
                    .wrap_err(format!(
                        "failed to get part 2 response for participant with id {:?}",
                        id
                    ))?;
                Ok((id, resp.into_inner()))
            });
        }
        let results: Vec<eyre::Result<_>> = stream.collect::<Vec<_>>().await;
        let mut sig_shares: BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> =
            BTreeMap::new();
        for res in results {
            if let Ok((id, part2)) = res {
                sig_shares.insert(
                    *id,
                    frost_ed25519::round2::SignatureShare::deserialize(&part2.signature_share)
                        .wrap_err("failed to deserialize signature share")?,
                );
            };
        }
        ensure!(
            sig_shares.len() >= min_signers,
            "not enough part 2 signature shares"
        );

        // aggregate and create signature
        let signing_package =
            frost_ed25519::SigningPackage::new(signing_package_commitments, &tx_bytes);
        let signature =
            frost_ed25519::aggregate(&signing_package, &sig_shares, &self.public_key_package)
                .wrap_err("failed to aggregate")?;

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
            .wrap_err("failed to convert to transaction")?;

        Ok(transaction)
    }

    fn address(&self) -> &Address {
        &self.address
    }
}
