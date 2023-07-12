use std::collections::HashMap;

use astria_celestia_jsonrpc_client::{
    blob::{
        self,
        GetAllRequest,
    },
    state,
    Client,
};
use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use eyre::{
    bail,
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
use tendermint::block::{
    Commit,
    Header,
};
use tracing::{
    debug,
    warn,
};

use crate::{
    base64_string::Base64String,
    types::{
        IndexedTransaction,
        Namespace,
        SequencerBlockData,
        DEFAULT_NAMESPACE,
    },
};

pub const DEFAULT_PFD_GAS_LIMIT: u64 = 1_000_000;
const DEFAULT_PFD_FEE: u128 = 2_000;

/// SubmitBlockResponse is the response to a SubmitBlock request.
/// It contains a map of namespaces to the block number that it was written to.
pub struct SubmitBlockResponse {
    /// the height the base namespace was written to
    pub height: u64,
    pub namespace_to_block_num: HashMap<Namespace, u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignedNamespaceData<D> {
    pub data: D,
    pub public_key: Base64String,
    pub signature: Base64String,
}

impl<D: NamespaceData> SignedNamespaceData<D> {
    fn new(data: D, public_key: Base64String, signature: Base64String) -> Self {
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
        let verification_key = VerificationKey::try_from(&*self.public_key.0)
            .wrap_err("failed deserializing public key from bytes")?;
        let signature = Signature::try_from(&*self.signature.0)
            .wrap_err("failed deserializing signature from bytes")?;
        let data_bytes = self
            .data
            .hash()
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
    fn hash(&self) -> eyre::Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(
            self.to_bytes()
                .wrap_err("failed converting namespace data to bytes")?,
        );
        let hash = hasher.finalize();
        Ok(hash.to_vec())
    }

    fn to_signed(
        self,
        signing_key: &SigningKey,
        verification_key: VerificationKey,
    ) -> eyre::Result<SignedNamespaceData<Self>> {
        let hash = self.hash().wrap_err("failed hashing namespace data")?;
        let signature = Base64String(signing_key.sign(&hash).to_bytes().to_vec());
        let data = SignedNamespaceData::new(
            self,
            Base64String(verification_key.to_bytes().into()),
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
    pub block_hash: Base64String,
    pub header: Header,
    pub last_commit: Option<Commit>,
    /// vector of (block height, namespace) tuples
    /// TODO: can get rid of block height when multiple
    /// blobs are written atomically
    pub rollup_namespaces: Vec<(u64, Namespace)>,
}

impl NamespaceData for SequencerNamespaceData {}

/// RollupNamespaceData represents the data written to a rollup namespace.
#[derive(Serialize, Deserialize, Debug)]
struct RollupNamespaceData {
    pub(crate) block_hash: Base64String,
    pub(crate) rollup_txs: Vec<IndexedTransaction>,
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
pub struct CelestiaClient {
    client: Client,
    gas_limit: u64,
    fee: u128,
}

impl CelestiaClient {
    pub fn builder() -> CelestiaClientBuilder {
        CelestiaClientBuilder::new()
    }

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
        namespace_id: [u8; blob::NAMESPACE_ID_AVAILABLE_LEN],
        data: &[u8],
    ) -> eyre::Result<state::SubmitPayForBlobResponse> {
        let req = state::SubmitPayForBlobRequest {
            fee: self.fee,
            gas_limit: self.gas_limit,
            blobs: vec![blob::Blob {
                namespace_id,
                data: data.to_vec(),
            }],
        };
        self.client
            .state_submit_pay_for_blob(req)
            .await
            .wrap_err("failed submitting pay for data to client")
    }

    /// submit_block submits a block to Celestia.
    /// It first writes all the rollup namespace data, then writes the sequencer namespace data.
    /// The sequencer namespace data contains all the rollup namespaces that were written,
    /// along with any transactions that were not for a specific rollup.
    pub async fn submit_block(
        &self,
        block: SequencerBlockData,
        signing_key: &SigningKey,
        verification_key: VerificationKey,
    ) -> eyre::Result<SubmitBlockResponse> {
        let mut namespace_to_block_num: HashMap<Namespace, u64> = HashMap::new();
        let mut block_height_and_namespace: Vec<(u64, Namespace)> = Vec::new();

        // first, format and submit data for each rollup namespace
        //
        // TODO: This could probably now be combined into one submission?
        for (namespace, txs) in block.rollup_txs {
            debug!(
                "submitting rollup namespace data for namespace {}",
                namespace
            );
            let rollup_namespace_data = RollupNamespaceData {
                block_hash: block.block_hash.clone(),
                rollup_txs: txs,
            };
            let rollup_data_bytes = rollup_namespace_data
                .to_signed(signing_key, verification_key)
                .wrap_err("failed signing rollup namespace data")?
                .to_bytes()
                .wrap_err("failed converting signed rollupdata namespace data to bytes")?;
            let rsp = self
                .submit_namespaced_data(*namespace, &rollup_data_bytes)
                .await
                .wrap_err("failed submitting signed rollup namespaced data")?;
            let height = rsp.height;
            namespace_to_block_num.insert(namespace, height);
            block_height_and_namespace.push((height, namespace))
        }

        // then, format and submit data to the base sequencer namespace
        let sequencer_namespace_data = SequencerNamespaceData {
            block_hash: block.block_hash.clone(),
            header: block.header,
            last_commit: block.last_commit,
            rollup_namespaces: block_height_and_namespace,
        };

        let bytes = sequencer_namespace_data
            .to_signed(signing_key, verification_key)
            .wrap_err("failed signing sequencer namespace data")?
            .to_bytes()
            .wrap_err("failed converting signed namespace data to bytes")?;
        // TODO: Could this also be thrown into the submission above?
        let rsp = self
            .submit_namespaced_data(*DEFAULT_NAMESPACE, &bytes)
            .await
            .wrap_err("failed submitting namespaced data")?;

        let height = rsp.height;
        namespace_to_block_num.insert(DEFAULT_NAMESPACE, height);
        Ok(SubmitBlockResponse {
            height,
            namespace_to_block_num,
        })
    }

    /// get_sequencer_namespace_data returns all the signed sequencer namespace data at a given
    /// height.
    pub async fn get_sequencer_namespace_data(
        &self,
        height: u64,
        verification_key: Option<VerificationKey>,
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

        // retrieve all sequencer data stored at this height
        // optionally, only find data that was signed by the given public key
        // NOTE: there should NOT be multiple datas with the same block hash and signer;
        // should we check here, or should the caller check?
        let sequencer_namespace_datas = rsp
            .blobs
            .into_iter()
            .filter_map(|blob| {
                let data =
                    SignedNamespaceData::<SequencerNamespaceData>::from_bytes(&blob.data).ok()?;
                let Some(verification_key) = verification_key else {
                    return Some(data);
                };

                let hash = data.data.hash().ok()?;
                let signature = Signature::try_from(&*data.signature.0).ok()?;
                verification_key.verify(&signature, &hash).ok()?;
                Some(data)
            })
            .collect::<Vec<_>>();
        Ok(sequencer_namespace_datas)
    }

    /// get_sequencer_block returns the full SequencerBlock (with all rollup data) for the given
    /// SequencerNamespaceData.
    pub async fn get_sequencer_block(
        &self,
        data: &SequencerNamespaceData,
        verification_key: Option<VerificationKey>,
    ) -> eyre::Result<SequencerBlockData> {
        let mut rollup_txs_map = HashMap::new();

        // for each rollup namespace, retrieve the corresponding rollup data
        'namespaces: for (height, rollup_namespace) in &data.rollup_namespaces {
            let rollup_txs = self
                .get_rollup_data_for_block(
                    &data.block_hash.0,
                    *rollup_namespace,
                    *height,
                    verification_key,
                )
                .await
                .wrap_err_with(|| {
                    format!(
                        "failed getting rollup data for block at height `{height}` in rollup \
                         namespace `{rollup_namespace}`"
                    )
                })?;
            let Some(rollup_txs) = rollup_txs else {
                // this shouldn't happen; if a sequencer block claims to have written data to some
                // rollup namespace, it should exist
                warn!("no rollup data found for namespace {rollup_namespace}");
                continue 'namespaces;
            };
            rollup_txs_map.insert(*rollup_namespace, rollup_txs);
        }

        Ok(SequencerBlockData {
            block_hash: data.block_hash.clone(),
            header: data.header.clone(),
            last_commit: data.last_commit.clone(),
            rollup_txs: rollup_txs_map,
        })
    }

    async fn get_rollup_data_for_block(
        &self,
        block_hash: &[u8],
        rollup_namespace: Namespace,
        height: u64,
        verification_key: Option<VerificationKey>,
    ) -> eyre::Result<Option<Vec<IndexedTransaction>>> {
        let req = GetAllRequest {
            height,
            namespace_ids: vec![*rollup_namespace],
        };
        let rsp = self
            .client
            .blob_get_all(req)
            .await
            .wrap_err("failed getting namespaced data")?;

        let mut rollup_datas = rsp
            .blobs
            .iter()
            .filter_map(|blob| {
                if let Ok(data) = SignedNamespaceData::<RollupNamespaceData>::from_bytes(&blob.data)
                {
                    Some(data)
                } else {
                    warn!("failed to deserialize rollup namespace data");
                    None
                }
            })
            .filter(|d| {
                let hash = match d.data.hash() {
                    Ok(hash) => hash,
                    Err(_) => return false,
                };

                match Signature::try_from(&*d.signature.0) {
                    Ok(sig) => {
                        let Some(verification_key) = verification_key else {
                            return true;
                        };
                        verification_key.verify(&sig, &hash).is_ok()
                    }
                    Err(_) => false,
                }
            })
            .filter(|d| d.data.block_hash.0 == block_hash);

        let Some(rollup_data_for_block) = rollup_datas.next() else {
            return Ok(None);
        };

        // there should NOT be multiple datas with the same block hash and signer
        if rollup_datas.next().is_some() {
            bail!("multiple rollup datas with the same block hash and signer");
        }

        Ok(Some(rollup_data_for_block.data.rollup_txs))
    }
}
