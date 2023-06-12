use std::{
    collections::HashMap,
    vec,
};

use astria_proto::sequencer::v1::{
    IndexedTransaction as RawIndexedTransaction,
    RollupNamespace,
    RollupNamespaceData as RawRollupNamespaceData,
    SequencerNamespaceData as RawSequencerNamespaceData,
    SignedNamespaceData as RawSignedNamespaceData,
};
use astria_rs_cnc::{
    CelestiaNodeClient,
    NamespacedSharesResponse,
    PayForDataResponse,
};
use axum::response::Result;
use ed25519_dalek::{
    ed25519::signature::Signature,
    Keypair,
    PublicKey,
    Signer,
    Verifier,
};
use eyre::{
    bail,
    WrapErr as _,
};
use prost::Message;
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
    block::Header,
    Hash,
};
use tendermint_proto::Protobuf;
use tracing::{
    debug,
    warn,
};

use crate::{
    base64_string::Base64String,
    sequencer_block::{
        IndexedTransaction,
        Namespace,
        SequencerBlock,
        DEFAULT_NAMESPACE,
    },
    // types::Header,
};

pub const DEFAULT_PFD_GAS_LIMIT: u64 = 1_000_000;
const DEFAULT_PFD_FEE: i64 = 2_000;

/// SubmitBlockResponse is the response to a SubmitBlock request.
/// It contains a map of namespaces to the block number that it was written to.
pub struct SubmitBlockResponse {
    /// the height the base namespace was written to
    pub height: u64,
    pub namespace_to_block_num: HashMap<String, u64>,
}

pub trait NamespaceData
where
    Self: Sized + Serialize + DeserializeOwned,
{
    // TODO: shouldnt this be impl Hash for NamespaceData? (the version of this that actually works)
    fn hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.to_bytes());
        let hash = hasher.finalize();
        hash.to_vec()
    }

    fn to_signed(self, keypair: &Keypair) -> SignedNamespaceData<Self> {
        let hash = self.hash();
        let signature = Base64String(keypair.sign(&hash).as_bytes().to_vec());
        SignedNamespaceData::new(
            self,
            Base64String(keypair.public.to_bytes().to_vec()),
            signature,
        )
    }

    fn from_bytes(bytes: &[u8]) -> eyre::Result<Self>;
    fn to_bytes(&self) -> Vec<u8>;
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

    fn from_proto(proto: RawSignedNamespaceData) -> eyre::Result<Self> {
        Ok(Self {
            data: D::from_bytes(&proto.data)?,
            public_key: Base64String::from_bytes(&proto.public_key),
            signature: Base64String::from_bytes(&proto.signature),
        })
    }

    fn to_proto(&self) -> eyre::Result<RawSignedNamespaceData> {
        Ok(RawSignedNamespaceData {
            data: self.data.to_bytes(),
            public_key: self.public_key.0.clone(),
            signature: self.signature.0.clone(),
        })
    }

    pub fn verify(&self) -> eyre::Result<()> {
        let public_key = PublicKey::from_bytes(&self.public_key.0)
            .wrap_err("failed deserializing public key from bytes")?;
        let signature = Signature::from_bytes(&self.signature.0)
            .wrap_err("failed deserializing signature from bytes")?;
        let data_bytes = self.data.hash();
        public_key
            .verify(&data_bytes, &signature)
            .wrap_err("failed verifying signature")?;
        Ok(())
    }
}

impl<D: NamespaceData> NamespaceData for SignedNamespaceData<D> {
    fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        Self::from_proto(RawSignedNamespaceData::decode(bytes)?)
    }

    fn to_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }
}

/// SequencerNamespaceData represents the data written to the "base"
/// sequencer namespace. It contains all the other namespaces that were
/// also written to in the same block.
#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub sequencer_txs: Vec<IndexedTransaction>,
    /// vector of (block height, namespace) tuples
    pub rollup_namespaces: Vec<(u64, String)>,
}

impl SequencerNamespaceData {
    fn from_proto(proto: RawSequencerNamespaceData) -> eyre::Result<Self> {
        let rollup_namespaces: Vec<(u64, String)> = proto
            .rollup_namespaces
            .iter()
            .filter_map(|runs| {
                Some((
                    runs.block_height,
                    String::from_utf8(runs.namespace.clone()).ok()?,
                ))
            })
            .collect();

        Ok(Self {
            block_hash: Hash::from_bytes(tendermint::hash::Algorithm::Sha256, &proto.block_hash)?,
            header: Header::try_from(proto.header.clone().unwrap())?, // TODO: static errors
            sequencer_txs: proto
                .sequencer_txs
                .into_iter()
                .map(IndexedTransaction::from_proto)
                .collect::<eyre::Result<Vec<_>>>()?,
            rollup_namespaces,
        })
    }

    fn to_proto(&self) -> eyre::Result<RawSequencerNamespaceData> {
        Ok(RawSequencerNamespaceData {
            block_hash: self.block_hash.encode_vec()?,
            header: Some(tendermint_proto::types::Header::from(self.header.clone())),
            sequencer_txs: self
                .sequencer_txs
                .iter()
                .map(IndexedTransaction::to_proto)
                .collect::<Result<Vec<RawIndexedTransaction>, _>>()?,
            rollup_namespaces: self
                .rollup_namespaces
                .iter()
                .map(|(block_height, namespace)| RollupNamespace {
                    block_height: *block_height,
                    namespace: namespace.clone().into_bytes(),
                })
                .collect(),
        })
    }
}

impl NamespaceData for SequencerNamespaceData {
    fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        Self::from_proto(RawSequencerNamespaceData::decode(bytes)?)
    }

    fn to_bytes(&self) -> Vec<u8> {
        RawSequencerNamespaceData::encode_to_vec(&self.to_proto().unwrap()) // TODO: this shouldn't be able to panic
    }
}

/// RollupNamespaceData represents the data written to a rollup namespace.
#[derive(Serialize, Deserialize, Debug)]
struct RollupNamespaceData {
    pub block_hash: Hash,
    pub rollup_txs: Vec<IndexedTransaction>,
}

impl RollupNamespaceData {
    fn from_proto(proto: RawRollupNamespaceData) -> eyre::Result<Self> {
        Ok(Self {
            block_hash: Hash::from_bytes(tendermint::hash::Algorithm::Sha256, &proto.block_hash)?,
            rollup_txs: proto
                .rollup_txs
                .into_iter()
                .map(IndexedTransaction::from_proto)
                .collect::<eyre::Result<Vec<_>>>()?,
        })
    }

    fn to_proto(&self) -> eyre::Result<RawRollupNamespaceData> {
        Ok(RawRollupNamespaceData {
            block_hash: self.block_hash.encode_vec()?,
            rollup_txs: self
                .rollup_txs
                .iter()
                .map(IndexedTransaction::to_proto)
                .collect::<Result<Vec<RawIndexedTransaction>, _>>()?,
        })
    }
}

impl NamespaceData for RollupNamespaceData {
    fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        Self::from_proto(RawRollupNamespaceData::decode(bytes)?)
    }

    fn to_bytes(&self) -> Vec<u8> {
        RawRollupNamespaceData::encode_to_vec(&self.to_proto().unwrap()) // TODO: this shouldn't be able to panic
    }
}

pub struct CelestiaClientBuilder {
    endpoint: String,
    gas_limit: Option<u64>,
}

impl CelestiaClientBuilder {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            gas_limit: None,
        }
    }

    pub fn build(self) -> eyre::Result<CelestiaClient> {
        let cnc = CelestiaNodeClient::builder()
            .base_url(self.endpoint)
            .wrap_err("failed to set base URL for celestia node client; bad URL?")?
            .build()
            .wrap_err("failed creating celestia node client")?;
        Ok(CelestiaClient {
            client: cnc,
            gas_limit: self.gas_limit.unwrap_or(DEFAULT_PFD_GAS_LIMIT),
        })
    }

    /// sets the gas limit to be used for PFDs.
    pub fn gas_limit(self, gas_limit: u64) -> Self {
        Self {
            gas_limit: Some(gas_limit),
            ..self
        }
    }
}

/// CelestiaClient is a DataAvailabilityClient that submits blocks to a Celestia Node.
pub struct CelestiaClient {
    client: CelestiaNodeClient,
    gas_limit: u64,
}

impl CelestiaClient {
    pub async fn get_latest_height(&self) -> eyre::Result<u64> {
        let res = self
            .client
            .namespaced_data(&DEFAULT_NAMESPACE.to_string(), 0)
            .await
            .wrap_err("failed requesting namespaced data")?;
        let Some(height) = res.height else {
            bail!("no height found in namespaced data received by celestia client");
        };
        Ok(height)
    }

    async fn submit_namespaced_data(
        &self,
        namespace: &str,
        data: &[u8],
    ) -> eyre::Result<PayForDataResponse> {
        let pay_for_data_response = self
            .client
            .submit_pay_for_data(
                namespace,
                &data.to_vec().into(),
                DEFAULT_PFD_FEE,
                self.gas_limit,
            )
            .await
            .wrap_err("failed submitting pay for data to client")?;
        Ok(pay_for_data_response)
    }

    /// submit_block submits a block to Celestia.
    /// It first writes all the rollup namespace data, then writes the sequencer namespace data.
    /// The sequencer namespace data contains all the rollup namespaces that were written,
    /// along with any transactions that were not for a specific rollup.
    pub async fn submit_block(
        &self,
        block: SequencerBlock,
        keypair: &Keypair,
    ) -> eyre::Result<SubmitBlockResponse> {
        let mut namespace_to_block_num: HashMap<String, u64> = HashMap::new();
        let mut block_height_and_namespace: Vec<(u64, String)> = Vec::new();

        // first, format and submit data for each rollup namespace
        for (namespace, txs) in block.rollup_txs {
            debug!(
                "submitting rollup namespace data for namespace {}",
                namespace
            );
            let rollup_namespace_data = RollupNamespaceData {
                block_hash: block.block_hash.clone(),
                rollup_txs: txs,
            };
            let rollup_data_bytes = rollup_namespace_data.to_signed(keypair).to_bytes();
            let resp = self
                .submit_namespaced_data(&namespace.to_string(), &rollup_data_bytes)
                .await
                .wrap_err("failed submitting signed rollup namespaced data")?;

            let Some(height) = resp.height else {
                bail!("no height found in namespaced data received by celestia client");
            };
            let namespace = namespace.to_string();
            namespace_to_block_num.insert(namespace.clone(), height);
            block_height_and_namespace.push((height, namespace))
        }

        // then, format and submit data to the base sequencer namespace
        let sequencer_namespace_data = SequencerNamespaceData {
            block_hash: block.block_hash.clone(),
            header: block.header,
            sequencer_txs: block.sequencer_txs,
            rollup_namespaces: block_height_and_namespace,
        };

        let bytes = sequencer_namespace_data.to_signed(keypair).to_bytes();
        let resp = self
            .submit_namespaced_data(&DEFAULT_NAMESPACE.to_string(), &bytes)
            .await
            .wrap_err("failed submitting namespaced data")?;

        let Some(height) = resp.height else {
            bail!("no height returned from pay for data");
        };

        namespace_to_block_num.insert(DEFAULT_NAMESPACE.to_string(), height);

        Ok(SubmitBlockResponse {
            height,
            namespace_to_block_num,
        })
    }

    /// check_block_availability checks if what shares are written to a given height.
    pub async fn check_block_availability(
        &self,
        height: u64,
    ) -> eyre::Result<NamespacedSharesResponse> {
        let resp = self
            .client
            .namespaced_shares(&DEFAULT_NAMESPACE.to_string(), height)
            .await
            .wrap_err("failed accessing namespaced shares")?;
        Ok(resp)
    }

    /// get_sequencer_namespace_data returns all the signed sequencer namespace data at a given
    /// height.
    pub async fn get_sequencer_namespace_data(
        &self,
        height: u64,
        public_key: Option<&PublicKey>,
    ) -> eyre::Result<Vec<SignedNamespaceData<SequencerNamespaceData>>> {
        let namespaced_data_response = self
            .client
            .namespaced_data(&DEFAULT_NAMESPACE.to_string(), height)
            .await
            .wrap_err("failed getting namespaced data")?;

        // retrieve all sequencer data stored at this height
        // optionally, only find data that was signed by the given public key
        // NOTE: there should NOT be multiple datas with the same block hash and signer;
        // should we check here, or should the caller check?
        let sequencer_namespace_datas = namespaced_data_response
            .data
            .unwrap_or_default()
            .iter()
            .filter_map(|d| {
                let data = SignedNamespaceData::<SequencerNamespaceData>::from_bytes(&d.0).ok()?;
                let Some(public_key) = public_key else {
                    return Some(data);
                };

                let hash = data.data.hash();
                let signature = Signature::from_bytes(&data.signature.0).ok()?;
                public_key.verify(&hash, &signature).ok()?;
                Some(data)
            })
            .collect::<Vec<_>>();
        Ok(sequencer_namespace_datas)
    }

    /// get_blocks retrieves all blocks written to Celestia at the given height.
    /// If a public key is provided, it will only return blocks signed by that public key.
    /// It might return multiple blocks, because there might be multiple written to
    /// the same height.
    /// The caller should probably check that there are no conflicting blocks.
    pub async fn get_blocks(
        &self,
        height: u64,
        public_key: Option<&PublicKey>,
    ) -> eyre::Result<Vec<SequencerBlock>> {
        let sequencer_namespace_datas = self
            .get_sequencer_namespace_data(height, public_key)
            .await?;
        let mut blocks = Vec::with_capacity(sequencer_namespace_datas.len());

        // for all the sequencer datas retrieved, create the corresponding SequencerBlock
        for sequencer_namespace_data in &sequencer_namespace_datas {
            blocks.push(
                self.get_sequencer_block(&sequencer_namespace_data.data, public_key)
                    .await?,
            );
        }

        Ok(blocks)
    }

    /// get_sequencer_block returns the full SequencerBlock (with all rollup data) for the given
    /// SequencerNamespaceData.
    pub async fn get_sequencer_block(
        &self,
        data: &SequencerNamespaceData,
        public_key: Option<&PublicKey>,
    ) -> eyre::Result<SequencerBlock> {
        let mut rollup_txs_map = HashMap::new();

        // for each rollup namespace, retrieve the corresponding rollup data
        'namespaces: for (height, rollup_namespace) in &data.rollup_namespaces {
            let rollup_txs = self
                .get_rollup_data_for_block(
                    &data.block_hash, // TODO: change to Hash
                    rollup_namespace,
                    *height,
                    public_key,
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
            let namespace = Namespace::from_string(rollup_namespace).wrap_err_with(|| {
                format!("failed constructing namespaces from rollup namespace `{rollup_namespace}`")
            })?;
            rollup_txs_map.insert(namespace, rollup_txs);
        }

        Ok(SequencerBlock {
            block_hash: data.block_hash.clone(),
            header: data.header.clone(),
            sequencer_txs: data.sequencer_txs.clone(),
            rollup_txs: rollup_txs_map,
        })
    }

    async fn get_rollup_data_for_block(
        &self,
        block_hash: &Hash,
        rollup_namespace: &str,
        height: u64,
        public_key: Option<&PublicKey>,
    ) -> eyre::Result<Option<Vec<IndexedTransaction>>> {
        let namespaced_data_response = self
            .client
            .namespaced_data(rollup_namespace, height)
            .await
            .wrap_err("failed getting namespaced data")?;

        let datas = namespaced_data_response.data.unwrap_or_default();
        let mut rollup_datas = datas
            .iter()
            .filter_map(|d| {
                if let Ok(data) = SignedNamespaceData::<RollupNamespaceData>::from_bytes(&d.0) {
                    Some(data)
                } else {
                    warn!("failed to deserialize rollup namespace data");
                    None
                }
            })
            .filter(|d| {
                let hash = d.data.hash();
                match Signature::from_bytes(&d.signature.0) {
                    Ok(sig) => {
                        let Some(public_key) = public_key else {
                            return true;
                        };
                        public_key.verify(&hash, &sig).is_ok()
                    }
                    Err(_) => false,
                }
            })
            .filter(|d| d.data.block_hash == *block_hash);

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
