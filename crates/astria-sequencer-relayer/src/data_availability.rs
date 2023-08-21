use std::collections::{
    BTreeMap,
    HashMap,
};

use astria_celestia_jsonrpc_client::{
    blob::{
        self,
        GetAllRequest,
    },
    state,
    Client,
    ErrorKind,
};
use astria_sequencer_types::{
    serde::Base64Standard,
    Namespace,
    RollupData,
    SequencerBlockData,
    DEFAULT_NAMESPACE,
};
use astria_sequencer_validation::{
    generate_action_tree_leaves,
    InclusionProof,
    MerkleTree,
};
use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use eyre::{
    ensure,
    WrapErr as _,
};
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use sha2::{
    Digest,
    Sha256,
};
use tendermint::{
    block::{
        Commit,
        Header,
    },
    Hash,
};
use tracing::{
    info,
    instrument,
    warn,
};

pub const DEFAULT_PFD_GAS_LIMIT: u64 = 1_000_000;
const DEFAULT_PFD_FEE: u128 = 100_000;

/// SubmitBlockResponse is the response to a SubmitBlock request.
pub struct SubmitBlockResponse {
    pub height: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignedNamespaceData<D> {
    pub data: D,
    #[serde(with = "Base64Standard")]
    pub public_key: Vec<u8>,
    #[serde(with = "Base64Standard")]
    pub signature: Vec<u8>,
}

impl<D: NamespaceData> SignedNamespaceData<D> {
    fn new(data: D, public_key: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            data,
            public_key,
            signature,
        }
    }

    fn to_bytes(&self) -> eyre::Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        serde_json::to_vec(self).wrap_err("failed serializing signed namespace data to json")
    }

    fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        serde_json::from_slice(bytes)
            .wrap_err("failed deserializing signed namespace data from bytes")
    }

    pub fn verify(&self) -> eyre::Result<()> {
        let verification_key = VerificationKey::try_from(&*self.public_key)
            .wrap_err("failed deserializing public key from bytes")?;
        let signature = Signature::try_from(&*self.signature)
            .wrap_err("failed deserializing signature from bytes")?;
        let data_bytes = self
            .data
            .hash_json_serialized_bytes()
            .wrap_err("failed converting data to bytes")?;
        verification_key
            .verify(&signature, &data_bytes)
            .wrap_err("failed verifying signature")?;
        Ok(())
    }
}

pub trait NamespaceData
where
    Self: Sized + Serialize + DeserializeOwned,
{
    fn hash_json_serialized_bytes(&self) -> eyre::Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(
            self.to_bytes()
                .wrap_err("failed converting namespace data to bytes")?,
        );
        let hash = hasher.finalize();
        Ok(hash.to_vec())
    }

    fn to_signed(self, signing_key: &SigningKey) -> eyre::Result<SignedNamespaceData<Self>> {
        let hash = self
            .hash_json_serialized_bytes()
            .wrap_err("failed hashing namespace data")?;
        let signature = signing_key.sign(&hash).to_bytes().to_vec();
        let data = SignedNamespaceData::new(
            self,
            signing_key.verification_key().to_bytes().to_vec(),
            signature,
        );
        Ok(data)
    }

    fn to_bytes(&self) -> eyre::Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        serde_json::to_vec(self).wrap_err("failed serializing namespace data as json bytes")
    }
}

/// SequencerNamespaceData represents the data written to the "base"
/// sequencer namespace. It contains all the other namespaces that were
/// also written to in the same block.
#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub last_commit: Option<Commit>,
    pub rollup_namespaces: Vec<Namespace>,
    pub action_tree_root: Hash,
    pub action_tree_root_inclusion_proof: InclusionProof,
}

impl NamespaceData for SequencerNamespaceData {}

/// RollupNamespaceData represents the data written to a rollup namespace.
#[derive(Serialize, Deserialize, Debug)]
pub struct RollupNamespaceData {
    pub(crate) block_hash: Hash,
    pub chain_id: Vec<u8>,
    pub rollup_txs: Vec<Vec<u8>>,
    pub inclusion_proof: InclusionProof,
}

impl NamespaceData for RollupNamespaceData {}

#[derive(Debug)]
pub struct CelestiaClientBuilder {
    endpoint: Option<String>,
    bearer_token: Option<String>,
    gas_limit: u64,
    fee: u128,
}

impl Default for CelestiaClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CelestiaClientBuilder {
    /// Create a celestia client builder with its fields default initialized.
    pub(crate) fn new() -> Self {
        Self {
            endpoint: None,
            bearer_token: None,
            gas_limit: DEFAULT_PFD_GAS_LIMIT,
            fee: DEFAULT_PFD_FEE,
        }
    }

    pub fn endpoint(self, endpoint: &str) -> Self {
        Self {
            endpoint: Some(endpoint.to_string()),
            ..self
        }
    }

    pub fn bearer_token(self, bearer_token: &str) -> Self {
        Self {
            bearer_token: Some(bearer_token.to_string()),
            ..self
        }
    }

    pub fn gas_limit(self, gas_limit: u64) -> Self {
        Self {
            gas_limit,
            ..self
        }
    }

    pub fn fee(self, fee: u128) -> Self {
        Self {
            fee,
            ..self
        }
    }

    pub fn build(self) -> eyre::Result<CelestiaClient> {
        let Self {
            endpoint,
            bearer_token,
            gas_limit,
            fee,
        } = self;
        let client = {
            Client::builder()
                .set_endpoint(endpoint)
                .set_bearer_token(bearer_token)
                .build()
                .wrap_err("failed constructing a celestia jsonrpc client")?
        };
        Ok(CelestiaClient {
            client,
            gas_limit,
            fee,
        })
    }
}

/// CelestiaClient is a DataAvailabilityClient that submits blocks to a Celestia Node.
#[derive(Clone, Debug)]
pub struct CelestiaClient {
    client: Client,
    gas_limit: u64,
    fee: u128,
}

impl CelestiaClient {
    pub fn builder() -> CelestiaClientBuilder {
        CelestiaClientBuilder::new()
    }

    #[instrument(skip_all)]
    pub async fn get_latest_height(&self) -> eyre::Result<u64> {
        let res = self
            .client
            .header_network_head()
            .await
            .wrap_err("failed calling getting network head of celestia")?;
        Ok(res.height())
    }

    async fn submit_namespaced_data(
        &self,
        blobs: Vec<blob::Blob>,
    ) -> eyre::Result<state::SubmitPayForBlobResponse> {
        let req = state::SubmitPayForBlobRequest {
            fee: self.fee,
            gas_limit: self.gas_limit,
            blobs,
        };
        self.client
            .state_submit_pay_for_blob(req)
            .await
            .wrap_err("failed submitting pay for data to client")
    }

    /// Submit all `blocks` to the data availability layer in an atomic operation.
    ///
    /// Each block gets converted into a collection of blobs. If this conversion fails
    /// the block is dropped, emitting a tracing warning.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC failed.
    pub async fn submit_all_blocks(
        &self,
        blocks: Vec<SequencerBlockData>,
        signing_key: &SigningKey,
    ) -> eyre::Result<SubmitBlockResponse> {
        // The number of total expected blobs is:
        // + the sum of all rollup transactions in all blocks (each converted to a rollup namespaced
        //   data), and
        // + one sequencer namespaced data blob per block.
        let num_expected_blobs = blocks
            .iter()
            .map(|block| block.rollup_data().len() + 1)
            .sum();
        let mut all_blobs = Vec::with_capacity(num_expected_blobs);
        for block in blocks {
            match assemble_blobs_from_sequencer_block_data(block, signing_key) {
                Ok(mut blobs) => {
                    all_blobs.append(&mut blobs);
                }
                Err(e) => {
                    warn!(e.msg = %e, e.cause_chain = ?e, "failed assembling blobs from sequencer block data; skipping");
                }
            };
        }

        info!(
            num_blobs = all_blobs.len(),
            "calling rpc with converted sequencer blocks converted to celestia blobs",
        );
        let rsp = self
            .submit_namespaced_data(all_blobs)
            .await
            .wrap_err("failed submitting namespaced data to data availability layer")?;
        let height = rsp.height;
        Ok(SubmitBlockResponse {
            height,
        })
    }

    /// get sequencer namespace data for the default sequencer namespace at a given height
    pub async fn get_sequencer_namespace_data(
        &self,
        height: u64,
    ) -> eyre::Result<Vec<SignedNamespaceData<SequencerNamespaceData>>> {
        let req = GetAllRequest {
            height,
            namespace_ids: vec![*DEFAULT_NAMESPACE],
        };
        let rsp = self
            .client
            .blob_get_all(req)
            .await
            .wrap_err("failed getting namespaced data")?;
        let sequencer_namespace_datas = rsp
            .blobs
            .into_iter()
            .filter_map(|blob| {
                match SignedNamespaceData::<SequencerNamespaceData>::from_bytes(&blob.data) {
                    Ok(data) => Some(data),
                    Err(e) => {
                        warn!(error.msg = %e, error.cause_chain = ?e, "failed deserializing sequencer namespace data from bytes stored in retrieved celestia blob");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        Ok(sequencer_namespace_datas)
    }

    /// Returns the rollup data for a given rollup namespace at a given height, if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// + the verification key could not be constructed from the data stored in `namespace_data`;
    /// + the RPC to fetch the blobs failed.
    pub async fn get_rollup_data(
        &self,
        height: u64,
        namespace_data: &SignedNamespaceData<SequencerNamespaceData>,
        rollup_namespace: Namespace,
    ) -> eyre::Result<Option<RollupNamespaceData>> {
        let verification_key = VerificationKey::try_from(&*namespace_data.public_key)
            .wrap_err("failed constructing verification key from stored bytes")?;

        let req = GetAllRequest {
            height,
            namespace_ids: vec![*rollup_namespace],
        };

        let rsp = match self.client.blob_get_all(req).await {
            Ok(rsp) => rsp,
            Err(err)
                if err
                    .jsonrpc_response()
                    .map_or(false, |err| err.message().contains("blob: not found")) =>
            {
                return Ok(None);
            }
            Err(err) => {
                return Err(err).wrap_err("failed getting namespaced data");
            }
        };

        // filter out blobs; we should only be left with either zero or one rollup datas
        let rollup_datas = filter_and_convert_rollup_data_blobs(
            rsp.blobs,
            namespace_data.data.block_hash,
            &verification_key,
        );

        // this should *not* happen; the only case where it would happen is if the sequencer-relayer
        // posts multiple blobs with the same rollup ID for the same block (could this happen?)
        ensure!(
            rollup_datas.len() <= 1,
            "should not have more than one rollup data for the given block hash",
        );

        // this case can happen if someone posts a blob to the namespace with invalid data
        if rollup_datas.is_empty() {
            return Ok(None);
        }

        Ok(rollup_datas.into_iter().next().map(|data| data.1.data))
    }

    /// Returns all rollup data for the namespaces recorded in sequencer namespace data.
    ///
    /// This function queries the data availability layer for blobs submitted to the namespaces
    /// listed in `namespace_data`. It then filters those rollup datas that have block
    /// hashes corresponding to those in `namespace_data`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// + the verification key could not be constructed from the data stored in `namespace_data`;
    /// + the RPC to fetch the blobs failed.
    pub async fn get_all_rollup_data_from_sequencer_namespace_data(
        &self,
        height: u64,
        namespace_data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> eyre::Result<Option<SequencerBlockData>> {
        let verification_key = VerificationKey::try_from(&*namespace_data.public_key)
            .wrap_err("failed constructing verification key from stored bytes")?;

        let namespace_ids = namespace_data
            .data
            .rollup_namespaces
            .iter()
            .map(|ns| **ns)
            .collect();
        let req = GetAllRequest {
            height,
            namespace_ids,
        };
        let rsp = match self.client.blob_get_all(req).await {
            Ok(rsp) => rsp,
            Err(e) => {
                if let ErrorKind::Rpc(astria_celestia_jsonrpc_client::JsonRpseeError::Call(inner)) =
                    e.kind()
                {
                    if inner.message().contains("blob: not found") {
                        info!("could not find blobs under the listed namespaces");
                        return Ok(None);
                    }
                }
                return Err(e).wrap_err("failed getting namespaced data");
            }
        };

        let rollup_datas = filter_and_convert_rollup_data_blobs(
            rsp.blobs,
            namespace_data.data.block_hash,
            &verification_key,
        );

        // finally, extract the rollup txs from the rollup datas
        let rollup_txs = rollup_datas
            .into_iter()
            .map(|(namespace, rollup_datas)| {
                (
                    namespace,
                    RollupData {
                        chain_id: rollup_datas.data.chain_id,
                        transactions: rollup_datas.data.rollup_txs,
                    },
                )
            })
            .collect();
        Ok(Some(
            SequencerBlockData::new(
                namespace_data.data.block_hash,
                namespace_data.data.header.clone(),
                namespace_data.data.last_commit.clone(),
                rollup_txs,
                namespace_data.data.action_tree_root,
                namespace_data.data.action_tree_root_inclusion_proof.clone(),
            )
            .wrap_err("failed to construct SequencerBlockData from namespace data")?,
        ))
    }
}

/// Filters out blobs that cannot be deserialized to `SignedNamespaceData<RollupNamespaceData>`,
/// whose block hash is not the same as the one provided, whose data was signed with a public key
/// that does not match the one provided, and whose signature cannot be verified
/// with the provided verification key.
fn filter_and_convert_rollup_data_blobs(
    blobs: Vec<blob::Blob>,
    block_hash: Hash,
    verification_key: &VerificationKey,
) -> HashMap<Namespace, SignedNamespaceData<RollupNamespaceData>> {
    // get only rollup datas that can be deserialized
    let mut rollup_datas: HashMap<_, _> = blobs
        .iter()
        .filter_map(|blob| {
            if let Ok(data) = SignedNamespaceData::<RollupNamespaceData>::from_bytes(&blob.data) {
                Some((Namespace::new(blob.namespace_id), data))
            } else {
                // FIXME: provide some info to identify the rollup namespace data?
                warn!("failed to deserialize rollup namespace data");
                None
            }
        })
        .collect();

    // retain rollup datas whose block hash matches the block hash of the namespaced data
    rollup_datas.retain(|_, rollup_data| block_hash == rollup_data.data.block_hash);

    // retain rollup datas with public key matching that expected
    rollup_datas.retain(|_, rollup_data| verification_key.as_ref() == rollup_data.public_key);

    // retain rollup datas that can be verified
    rollup_datas.retain(|namespace, rollup_data| {
            if let Err(e) = rollup_data
                .data
                .hash_json_serialized_bytes()
                .wrap_err("failed hashing json serialized rollup namespace data")
                .and_then(|hash| {
                    Signature::try_from(&*rollup_data.signature)
                        .map(|signature| (hash, signature))
                        .wrap_err(
                            "failed constructing signature from signature bytes of namespace data",
                        )
                })
                .and_then(|(hash, signature)| {
                    verification_key.verify(&signature, &hash).wrap_err(
                        "applying verification key to signature and hash generated from rollup \
                         namespace data failed",
                    )
                })
            {
                warn!(error.msg = %e, error.cause = ?e, %namespace, "failed verifying rollup namespace data");
                return false;
            }
            true
        });

    rollup_datas
}

fn btree_from_rollup_data(
    rollup_data: HashMap<Namespace, RollupData>,
) -> BTreeMap<Vec<u8>, Vec<Vec<u8>>> {
    let mut btree = BTreeMap::new();
    for (_, data) in rollup_data {
        btree.insert(data.chain_id, data.transactions);
    }
    btree
}

fn assemble_blobs_from_sequencer_block_data(
    block_data: SequencerBlockData,
    signing_key: &SigningKey,
) -> eyre::Result<Vec<blob::Blob>> {
    let mut blobs = Vec::with_capacity(block_data.rollup_data().len() + 1);
    let mut namespaces = Vec::with_capacity(block_data.rollup_data().len() + 1);

    let (
        block_hash,
        header,
        last_commit,
        rollup_data,
        action_tree_root,
        action_tree_root_inclusion_proof,
    ) = block_data.into_values();

    let chain_id_to_txs = btree_from_rollup_data(rollup_data);
    let action_tree_leaves = generate_action_tree_leaves(&chain_id_to_txs);
    let action_tree = MerkleTree::from_leaves(action_tree_leaves);

    for (i, (chain_id, transactions)) in chain_id_to_txs.into_iter().enumerate() {
        let inclusion_proof = action_tree
            .prove_inclusion(i)
            .map_err(|e| eyre::eyre!(e))
            .context("failed to generate inclusion proof")?;

        let rollup_namespace_data = RollupNamespaceData {
            block_hash,
            chain_id: chain_id.clone(),
            rollup_txs: transactions,
            inclusion_proof,
        };

        let blob_data = rollup_namespace_data
            .to_signed(signing_key)
            .wrap_err("failed signing rollup namespace data")?
            .to_bytes()
            .wrap_err("failed converting signed rollup data namespace data to bytes")?;

        let namespace = Namespace::from_slice(&chain_id);
        blobs.push(blob::Blob {
            namespace_id: *namespace,
            data: blob_data,
        });
        namespaces.push(namespace);
    }

    let sequencer_namespace_data = SequencerNamespaceData {
        block_hash,
        header,
        last_commit,
        rollup_namespaces: namespaces,
        action_tree_root,
        action_tree_root_inclusion_proof,
    };

    let data = sequencer_namespace_data
        .to_signed(signing_key)
        .wrap_err("failed signing sequencer namespace data")?
        .to_bytes()
        .wrap_err("failed converting signed namespace data to bytes")?;

    blobs.push(blob::Blob {
        namespace_id: *DEFAULT_NAMESPACE,
        data,
    });
    Ok(blobs)
}
