use async_trait::async_trait;
use celestia_rpc::BlobClient;
use celestia_types::{
    blob::SubmitOptions,
    nmt::Namespace,
    Blob,
    Commitment,
};
use ed25519_consensus::SigningKey;
use sequencer_types::{
    RawSequencerBlockData,
    SequencerBlockData,
};
use sequencer_validation::IndexOutOfBounds;

use crate::{
    blob_space::{
        celestia_namespace_v0_from_hashed_bytes,
        SequencerNamespaceData,
        SignedNamespaceData,
    },
    RollupNamespaceData,
};

impl CelestiaClientExt for jsonrpsee::http_client::HttpClient {}

#[derive(Debug, thiserror::Error)]
pub enum SubmitSequencerBlocksError {
    #[error("failed assembling blob for block at index `{index}`")]
    AssembleBlobs {
        source: BlobAssemblyError,
        index: usize,
    },
    #[error("the JSONRPC call failed")]
    JsonRpc(#[source] jsonrpsee::core::Error),
}

pub struct BadBlob {
    pub reason: BadBlobReason,
    pub commitment: Commitment,
}

pub enum BadBlobReason {
    Deserialization(serde_json::Error),
    WrongNamespace(Namespace),
}

pub struct GetSequencerDataResponse {
    pub height: u64,
    pub namespace: Namespace,
    pub datas: Vec<SignedNamespaceData<SequencerNamespaceData>>,
    pub bad_blobs: Vec<BadBlob>,
}

pub struct GetRollupDataResponse {
    pub height: u64,
    pub namespace: Namespace,
    pub datas: Vec<SignedNamespaceData<RollupNamespaceData>>,
    pub bad_blobs: Vec<BadBlob>,
}

#[async_trait]
pub trait CelestiaClientExt: BlobClient {
    /// Fetch sequencer data at the given height with the provided namespace.
    ///
    /// Returns successfully deserialized blobs in the `.datas` field. The
    /// `.bad_blobs` field contains the celestia commitment for each blob
    /// that could not be turned into sequencer data and the reason for it.
    ///
    /// # Errors
    ///
    /// Fails if the underlying `blob.GetAll` JSONRPC failed.
    async fn get_sequencer_data<T>(
        &self,
        height: T,
        namespace: Namespace,
    ) -> Result<GetSequencerDataResponse, jsonrpsee::core::Error>
    where
        T: Into<u64> + Send,
    {
        let height = height.into();
        let blobs = self.blob_get_all(height, &[namespace]).await?;

        let mut datas = Vec::new();
        let mut bad_blobs = Vec::new();
        for blob in blobs {
            if blob.namespace != namespace {
                bad_blobs.push(BadBlob {
                    reason: BadBlobReason::WrongNamespace(blob.namespace),
                    commitment: blob.commitment,
                });
            }
            match serde_json::from_slice(&blob.data) {
                Ok(data) => datas.push(data),
                Err(err) => bad_blobs.push(BadBlob {
                    reason: BadBlobReason::Deserialization(err),
                    commitment: blob.commitment,
                }),
            }
        }

        Ok(GetSequencerDataResponse {
            height,
            namespace,
            datas,
            bad_blobs,
        })
    }

    /// Returns the rollup data for a given rollup namespace at a given height, if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// + the verification key could not be constructed from the data stored in `namespace_data`;
    /// + the RPC to fetch the blobs failed.
    async fn get_rollup_data_matching_sequencer_data<T>(
        &self,
        height: T,
        namespace: Namespace,
        sequencer_data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> Result<Vec<RollupNamespaceData>, jsonrpsee::core::Error>
    where
        T: Into<u64> + Send,
    {
        #[must_use]
        fn is_blob_not_found(err: &jsonrpsee::core::Error) -> bool {
            if let jsonrpsee::core::Error::Call(err) = err {
                return err.message().contains("blob: not found");
            }
            false
        }

        let height = height.into();

        let rsp = self.blob_get_all(height, &[namespace]).await;
        let blobs = match rsp {
            Ok(blobs) => blobs,
            Err(err) if is_blob_not_found(&err) => {
                return Ok(vec![]);
            }
            Err(err) => {
                return Err(err);
            }
        };
        let rollup_datas = filter_and_convert_rollup_data_blobs(&blobs, namespace, sequencer_data);
        Ok(rollup_datas)
    }

    /// Submits sequencer `blocks` to celestia after converting and signing them, returning the
    /// height at which they were included.
    ///
    /// This calls the `blob.Submit` celestia-node RPC.
    async fn submit_sequencer_blocks(
        &self,
        namespace: Namespace,
        blocks: Vec<SequencerBlockData>,
        signing_key: &SigningKey,
        submit_options: SubmitOptions,
    ) -> Result<u64, SubmitSequencerBlocksError> {
        // The number of total expected blobs is:
        // + the sum of all rollup transactions in all blocks (each converted to a rollup namespaced
        //   data), and
        // + one sequencer namespaced data blob per block.
        let num_expected_blobs = blocks
            .iter()
            .fold(0, |acc, block| acc + block.rollup_data().len() + 1);

        let mut all_blobs = Vec::with_capacity(num_expected_blobs);
        for (i, block) in blocks.into_iter().enumerate() {
            let mut blobs = assemble_blobs_from_sequencer_block_data(namespace, block, signing_key)
                .map_err(|source| SubmitSequencerBlocksError::AssembleBlobs {
                    source,
                    index: i,
                })?;
            all_blobs.append(&mut blobs);
        }

        let height = self
            .blob_submit(&all_blobs, submit_options)
            .await
            .map_err(SubmitSequencerBlocksError::JsonRpc)?;

        Ok(height)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlobAssemblyError {
    #[error("failed constructing celestia blob from rollup data at index `{index}`")]
    ConstructBlobFromRollupData {
        source: celestia_types::Error,
        index: usize,
    },
    #[error("failed constructing celestia blob from sequencer data")]
    ConstructBlobFromSequencerData(#[source] celestia_types::Error),
    #[error("failed signing rollup namespace data at index `{index}`")]
    SignRollupData {
        source: serde_json::Error,
        index: usize,
    },
    #[error("failed to generate inclusion proof for the transaction at index `{index}`")]
    GenerateInclusionProof {
        source: IndexOutOfBounds,
        index: usize,
    },
}

fn assemble_blobs_from_sequencer_block_data(
    namespace: Namespace,
    block_data: SequencerBlockData,
    signing_key: &SigningKey,
) -> Result<Vec<Blob>, BlobAssemblyError> {
    use sequencer_validation::{
        generate_action_tree_leaves,
        MerkleTree,
    };

    let mut blobs = Vec::with_capacity(block_data.rollup_data().len() + 1);
    let mut chain_ids = Vec::with_capacity(block_data.rollup_data().len());

    let RawSequencerBlockData {
        block_hash,
        header,
        last_commit,
        rollup_data,
        action_tree_root,
        action_tree_root_inclusion_proof,
        chain_ids_commitment,
    } = block_data.into_raw();

    let action_tree_leaves = generate_action_tree_leaves(rollup_data.clone());
    let action_tree = MerkleTree::from_leaves(action_tree_leaves);

    for (i, (chain_id, transactions)) in rollup_data.into_iter().enumerate() {
        let inclusion_proof = action_tree.prove_inclusion(i).map_err(|source| {
            BlobAssemblyError::GenerateInclusionProof {
                source,
                index: i,
            }
        })?;

        let rollup_namespace_data = RollupNamespaceData {
            block_hash,
            chain_id: chain_id.clone(),
            rollup_txs: transactions,
            inclusion_proof,
        };

        let signed_data =
            SignedNamespaceData::from_data_and_key(rollup_namespace_data, signing_key);
        let data = serde_json::to_vec(&signed_data).expect(
            "should not fail because SignedNamespaceData and RollupNamespaceData do not contain \
             maps and hence non-unicode keys that would trigger to_vec()'s only error case",
        );

        let namespace = celestia_namespace_v0_from_hashed_bytes(chain_id.as_ref());
        blobs.push(Blob::new(namespace, data).map_err(|source| {
            BlobAssemblyError::ConstructBlobFromRollupData {
                source,
                index: i,
            }
        })?);
        chain_ids.push(chain_id);
    }

    let sequencer_namespace_data = SequencerNamespaceData {
        block_hash,
        header,
        last_commit,
        rollup_chain_ids: chain_ids,
        action_tree_root,
        action_tree_root_inclusion_proof,
        chain_ids_commitment,
    };

    let signed_data = SignedNamespaceData::from_data_and_key(sequencer_namespace_data, signing_key);
    let data = serde_json::to_vec(&signed_data).expect(
        "should not fail because SignedNamespaceData and SequencerNamespaceData do not contain \
         maps and hence non-unicode keys that would trigger to_vec()'s only error case",
    );

    blobs.push(
        Blob::new(namespace, data).map_err(BlobAssemblyError::ConstructBlobFromSequencerData)?,
    );
    Ok(blobs)
}

/// Filters out blobs that cannot be deserialized to `SignedNamespaceData<RollupNamespaceData>`,
/// whose block hash or public key do not match that of `sequencer_data`, respectively, or that
/// have the wrong namespace.
fn filter_and_convert_rollup_data_blobs(
    blobs: &[Blob],
    namespace: Namespace,
    sequencer_data: &SignedNamespaceData<SequencerNamespaceData>,
) -> Vec<RollupNamespaceData> {
    let mut rollups = Vec::with_capacity(blobs.len());
    let block_hash = sequencer_data.data().block_hash;
    let verification_key = sequencer_data.public_key();
    for blob in blobs {
        let Ok(data) =
            serde_json::from_slice::<SignedNamespaceData<RollupNamespaceData>>(&blob.data)
        else {
            continue;
        };
        if blob.namespace == namespace
            && data.data().block_hash == block_hash
            && data.public_key() == verification_key
        {
            rollups.push(data.into_unverified().data);
        }
    }
    rollups
}
