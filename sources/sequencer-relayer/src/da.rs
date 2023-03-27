use ed25519_dalek::{ed25519::signature::Signature, Keypair, PublicKey, Signer, Verifier};
use eyre::{eyre, Result};
use rs_cnc::{CelestiaNodeClient, NamespacedSharesResponse, PayForDataResponse};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::base64_string::Base64String;
use crate::sequencer_block::{IndexedTransaction, Namespace, SequencerBlock, DEFAULT_NAMESPACE};
use crate::types::Header;

static DEFAULT_PFD_FEE: i64 = 2_000;
static DEFAULT_PFD_GAS_LIMIT: u64 = 90_000;

/// SubmitBlockResponse is the response to a SubmitBlock request.
/// It contains a map of namespaces to the block number that it was written to.
pub struct SubmitBlockResponse {
    pub namespace_to_block_num: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SignedNamespaceData<D> {
    data: D,
    signature: Base64String,
}

impl<D: NamespaceData> SignedNamespaceData<D> {
    fn new(data: D, signature: Base64String) -> Self {
        Self { data, signature }
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        let string = serde_json::to_string(self).map_err(|e| eyre!(e))?;
        Ok(string.into_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let string = String::from_utf8(bytes.to_vec()).map_err(|e| eyre!(e))?;
        let data = serde_json::from_str(&string).map_err(|e| eyre!(e))?;
        Ok(data)
    }
}

trait NamespaceData
where
    Self: Sized + Serialize + DeserializeOwned,
{
    fn hash(&self) -> Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(self.to_bytes()?);
        let hash = hasher.finalize();
        Ok(hash.to_vec())
    }

    fn to_signed(self, keypair: &Keypair) -> Result<SignedNamespaceData<Self>> {
        let hash = self.hash()?;
        let signature = Base64String(keypair.sign(&hash).as_bytes().to_vec());
        let data = SignedNamespaceData::new(self, signature);
        Ok(data)
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        let string = serde_json::to_string(self).map_err(|e| eyre!(e))?;
        Ok(string.into_bytes())
    }
}

/// SequencerNamespaceData represents the data written to the "base"
/// sequencer namespace. It contains all the other namespaces that were
/// also written to in the same block.
#[derive(Serialize, Deserialize, Debug)]
struct SequencerNamespaceData {
    block_hash: Base64String,
    header: Header,
    sequencer_txs: Vec<IndexedTransaction>,
    /// vector of (block height, namespace) tuples
    rollup_namespaces: Vec<(u64, String)>,
}

impl NamespaceData for SequencerNamespaceData {}

/// RollupNamespaceData represents the data written to a rollup namespace.
#[derive(Serialize, Deserialize, Debug)]
struct RollupNamespaceData {
    block_hash: Base64String,
    rollup_txs: Vec<IndexedTransaction>,
}

impl NamespaceData for RollupNamespaceData {}

/// CelestiaClient is a DataAvailabilityClient that submits blocks to a Celestia Node.
pub struct CelestiaClient {
    client: CelestiaNodeClient,
}

impl CelestiaClient {
    /// new creates a new CelestiaClient with the given keypair.
    /// the keypair is used to sign the data that is submitted to Celestia,
    /// specifically within submit_block.
    pub fn new(endpoint: String) -> Result<Self> {
        let cnc = CelestiaNodeClient::new(endpoint).map_err(|e| eyre!(e))?;
        Ok(CelestiaClient { client: cnc })
    }

    async fn submit_namespaced_data(
        &self,
        namespace: &str,
        data: &[u8],
    ) -> Result<PayForDataResponse> {
        let pay_for_data_response = self
            .client
            .submit_pay_for_data(
                namespace,
                &data.to_vec().into(),
                DEFAULT_PFD_FEE,
                DEFAULT_PFD_GAS_LIMIT,
            )
            .await
            .map_err(|e| eyre!(e))?;
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
    ) -> Result<SubmitBlockResponse> {
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
            let rollup_data_bytes = rollup_namespace_data.to_signed(keypair)?.to_bytes()?;
            let resp = self
                .submit_namespaced_data(&namespace.to_string(), &rollup_data_bytes)
                .await?;

            if resp.height.is_none() {
                return Err(eyre!("no height returned from pay for data"));
            }

            namespace_to_block_num.insert(namespace.to_string(), resp.height.unwrap());
            block_height_and_namespace.push((resp.height.unwrap(), namespace.to_string()))
        }

        // then, format and submit data to the base sequencer namespace
        let sequencer_namespace_data = SequencerNamespaceData {
            block_hash: block.block_hash.clone(),
            header: block.header,
            sequencer_txs: block.sequencer_txs,
            rollup_namespaces: block_height_and_namespace,
        };

        let bytes = sequencer_namespace_data.to_signed(keypair)?.to_bytes()?;
        let resp = self
            .submit_namespaced_data(&DEFAULT_NAMESPACE.to_string(), &bytes)
            .await?;

        if resp.height.is_none() {
            return Err(eyre!("no height returned from pay for data"));
        }

        namespace_to_block_num.insert(DEFAULT_NAMESPACE.to_string(), resp.height.unwrap());

        Ok(SubmitBlockResponse {
            namespace_to_block_num,
        })
    }

    /// check_block_availability checks if what shares are written to a given height.
    pub async fn check_block_availability(&self, height: u64) -> Result<NamespacedSharesResponse> {
        let resp = self
            .client
            .namespaced_shares(&DEFAULT_NAMESPACE.to_string(), height)
            .await
            .map_err(|e| eyre!(e))?;
        Ok(resp)
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
    ) -> Result<Vec<SequencerBlock>> {
        let namespaced_data_response = self
            .client
            .namespaced_data(&DEFAULT_NAMESPACE.to_string(), height)
            .await
            .map_err(|e| eyre!(e))?;

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
                let hash = data.data.hash().ok()?;
                let signature = Signature::from_bytes(&data.signature.0).ok()?;
                let Some(public_key) = public_key else {
                    return Some(data);
                };
                public_key.verify(&hash, &signature).ok()?;
                Some(data)
            })
            .collect::<Vec<_>>();

        let mut blocks = Vec::with_capacity(sequencer_namespace_datas.len());

        // for all the sequencer datas retrieved, create the corresponding SequencerBlock
        for sequencer_namespace_data in &sequencer_namespace_datas {
            let mut rollup_txs_map = HashMap::new();

            // for each rollup namespace, retrieve the corresponding rollup data
            for (height, rollup_namespace) in &sequencer_namespace_data.data.rollup_namespaces {
                let rollup_txs = self
                    .get_rollup_data_for_block(
                        &sequencer_namespace_data.data.block_hash.0,
                        rollup_namespace,
                        *height,
                        public_key,
                    )
                    .await?;
                if rollup_txs.is_none() {
                    // this shouldn't happen; if a sequencer block claims to have written data to some
                    // rollup namespace, it should exist
                    warn!("no rollup data found for namespace {}", rollup_namespace);
                    continue;
                }
                rollup_txs_map.insert(
                    Namespace::from_string(rollup_namespace)?,
                    rollup_txs.unwrap(),
                );
            }

            blocks.push(SequencerBlock {
                block_hash: sequencer_namespace_data.data.block_hash.clone(),
                header: sequencer_namespace_data.data.header.clone(),
                sequencer_txs: sequencer_namespace_data.data.sequencer_txs.clone(),
                rollup_txs: rollup_txs_map,
            })
        }

        Ok(blocks)
    }

    async fn get_rollup_data_for_block(
        &self,
        block_hash: &[u8],
        rollup_namespace: &str,
        height: u64,
        public_key: Option<&PublicKey>,
    ) -> Result<Option<Vec<IndexedTransaction>>> {
        let namespaced_data_response = self
            .client
            .namespaced_data(rollup_namespace, height)
            .await
            .map_err(|e| eyre!(e))?;

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
                let hash = match d.data.hash() {
                    Ok(hash) => hash,
                    Err(_) => return false,
                };

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
            .filter(|d| d.data.block_hash.0 == block_hash);

        let Some(rollup_data_for_block) = rollup_datas.next() else {
            return Ok(None);
        };

        // there should NOT be multiple datas with the same block hash and signer
        if rollup_datas.next().is_some() {
            return Err(eyre!(
                "multiple rollup datas with the same block hash and signer"
            ));
        }

        Ok(Some(rollup_data_for_block.data.rollup_txs))
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Keypair, PublicKey};
    use rand::rngs::OsRng;
    use std::collections::HashMap;

    use super::{CelestiaClient, SequencerBlock, DEFAULT_NAMESPACE};
    use crate::base64_string::Base64String;
    use crate::sequencer_block::{get_namespace, IndexedTransaction};

    #[tokio::test]
    async fn test_get_blocks_public_key_filter() {
        // test that get_blocks only gets blocked signed with a specific key
        let keypair = Keypair::generate(&mut OsRng);
        let base_url = "http://localhost:26659".to_string();
        let client = CelestiaClient::new(base_url).unwrap();
        let tx = Base64String(b"noot_was_here".to_vec());

        let block_hash = Base64String(vec![99; 32]);
        let block = SequencerBlock {
            block_hash: block_hash.clone(),
            header: Default::default(),
            sequencer_txs: vec![IndexedTransaction {
                index: 0,
                transaction: tx.clone(),
            }],
            rollup_txs: HashMap::new(),
        };

        let submit_block_resp = client.submit_block(block, &keypair).await.unwrap();
        let height = submit_block_resp
            .namespace_to_block_num
            .get(&DEFAULT_NAMESPACE.to_string())
            .unwrap();

        // generate new, different key
        let keypair = Keypair::generate(&mut OsRng);
        let public_key = PublicKey::from_bytes(&keypair.public.to_bytes()).unwrap();
        let resp = client.get_blocks(*height, Some(&public_key)).await.unwrap();
        assert!(resp.is_empty());
    }

    #[tokio::test]
    async fn test_celestia_client() {
        // test submit_block
        let keypair = Keypair::generate(&mut OsRng);
        let public_key = PublicKey::from_bytes(&keypair.public.to_bytes()).unwrap();

        let base_url = "http://localhost:26659".to_string();
        let client = CelestiaClient::new(base_url).unwrap();
        let tx = Base64String(b"noot_was_here".to_vec());
        let secondary_namespace = get_namespace(b"test_namespace");
        let secondary_tx = Base64String(b"noot_was_here_too".to_vec());

        let block_hash = Base64String(vec![99; 32]);
        let mut block = SequencerBlock {
            block_hash: block_hash.clone(),
            header: Default::default(),
            sequencer_txs: vec![IndexedTransaction {
                index: 0,
                transaction: tx.clone(),
            }],
            rollup_txs: HashMap::new(),
        };
        block.rollup_txs.insert(
            secondary_namespace.clone(),
            vec![IndexedTransaction {
                index: 1,
                transaction: secondary_tx.clone(),
            }],
        );

        let submit_block_resp = client.submit_block(block, &keypair).await.unwrap();
        let height = submit_block_resp
            .namespace_to_block_num
            .get(&DEFAULT_NAMESPACE.to_string())
            .unwrap();

        // test check_block_availability
        let resp = client.check_block_availability(*height).await.unwrap();
        assert_eq!(resp.height, *height);

        // test get_blocks
        let resp = client.get_blocks(*height, Some(&public_key)).await.unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].block_hash, block_hash);
        assert_eq!(resp[0].header, Default::default());
        assert_eq!(resp[0].sequencer_txs.len(), 1);
        assert_eq!(resp[0].sequencer_txs[0].index, 0);
        assert_eq!(resp[0].sequencer_txs[0].transaction, tx);
        assert_eq!(resp[0].rollup_txs.len(), 1);
        assert_eq!(resp[0].rollup_txs[&secondary_namespace][0].index, 1);
        assert_eq!(
            resp[0].rollup_txs[&secondary_namespace][0].transaction,
            secondary_tx
        );
    }
}
