use ::metrics;
use astria_core::sequencer::v1alpha1::{
    celestia::CelestiaSequencerBlobError,
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
    SequencerBlock,
};
use async_trait::async_trait;
use base64::{
    display::Base64Display,
    engine::general_purpose::STANDARD,
};
use celestia_rpc::BlobClient;
use celestia_types::{
    blob::SubmitOptions,
    nmt::Namespace,
    Blob,
    Commitment,
};
use prost::{
    DecodeError,
    Message as _,
};
use tracing::{
    instrument,
    warn,
};

use crate::{
    celestia_namespace_v0_from_cometbft_header,
    celestia_namespace_v0_from_rollup_id,
    metrics as metric_labels,
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
    Conversion(CelestiaSequencerBlobError),
    Deserialization(DecodeError),
    WrongNamespace(Namespace),
}

pub struct GetSequencerBlobsResponse {
    pub height: u64,
    pub namespace: Namespace,
    pub sequencer_blobs: Vec<CelestiaSequencerBlob>,
    pub bad_blobs: Vec<BadBlob>,
}

#[async_trait]
pub trait CelestiaClientExt: BlobClient {
    /// Fetch sequencer blobs at the given height and namespace.
    ///
    /// Returns successfully deserialized blobs in the `.sequencer_blobs` field. The
    /// `.bad_blobs` field contains the celestia commitment for each blob
    /// that could not be turned into sequencer data and the reason for it.
    ///
    /// # Errors
    ///
    /// Fails if the underlying `blob.GetAll` JSONRPC failed.
    async fn get_sequencer_blobs<T>(
        &self,
        height: T,
        namespace: Namespace,
    ) -> Result<GetSequencerBlobsResponse, jsonrpsee::core::Error>
    where
        T: Into<u64> + Send,
    {
        let height = height.into();
        let blobs = self.blob_get_all(height, &[namespace]).await?;

        let mut sequencer_blobs = Vec::new();
        let mut bad_blobs = Vec::new();
        for blob in blobs {
            if blob.namespace != namespace {
                bad_blobs.push(BadBlob {
                    reason: BadBlobReason::WrongNamespace(blob.namespace),
                    commitment: blob.commitment,
                });
            }
            'blob: {
                let raw_blob =
                    match astria_core::generated::sequencer::v1alpha1::CelestiaSequencerBlob::decode(
                        &*blob.data,
                    ) {
                        Ok(blob) => blob,
                        Err(err) => {
                            bad_blobs.push(BadBlob {
                                reason: BadBlobReason::Deserialization(err),
                                commitment: blob.commitment,
                            });
                            break 'blob;
                        }
                    };
                match CelestiaSequencerBlob::try_from_raw(raw_blob) {
                    Ok(blob) => sequencer_blobs.push(blob),
                    Err(err) => bad_blobs.push(BadBlob {
                        reason: BadBlobReason::Conversion(err),
                        commitment: blob.commitment,
                    }),
                }
            }
        }

        Ok(GetSequencerBlobsResponse {
            height,
            namespace,
            sequencer_blobs,
            bad_blobs,
        })
    }

    /// Returns the rollup blob for a given rollup namespace at a given height, if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// + the verification key could not be constructed from the data stored in `namespace_data`;
    /// + the RPC to fetch the blobs failed.
    #[instrument(skip(self, height), fields(height = height.into()))]
    async fn get_rollup_blobs_matching_sequencer_blob<T>(
        &self,
        height: T,
        namespace: Namespace,
        sequencer_blob: &CelestiaSequencerBlob,
    ) -> Result<Vec<CelestiaRollupBlob>, jsonrpsee::core::Error>
    where
        T: Into<u64> + Copy + Send,
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
        let rollup_datas = convert_and_filter_rollup_blobs(blobs, namespace, sequencer_blob);
        Ok(rollup_datas)
    }

    /// Submits sequencer `blocks` to celestia
    ///
    /// `Blocks` after converted into celestia blobs and then posted. Rollup
    /// data is posted to a namespace derived from the rollup chain id.
    /// Sequencer data for each is posted to a namespace derived from the
    /// sequencer block's chain ID.
    ///
    /// This calls the `blob.Submit` celestia-node RPC.
    ///
    /// Returns Result:
    /// - Ok: the celestia block height blobs were included in.
    /// - Errors:
    ///     - SubmitSequencerBlocksError::AssembleBlobs when failed to assemble blob
    ///     - SubmitSequencerBlocksError::JsonRpc when Celestia `blob.Submit` fails
    async fn submit_sequencer_blocks(
        &self,
        blocks: Vec<SequencerBlock>,
        submit_options: SubmitOptions,
    ) -> Result<u64, SubmitSequencerBlocksError> {
        // The number of total expected blobs is:
        // + the sum of all rollup transactions in all blocks (each converted to a rollup namespaced
        //   data), and
        // + one sequencer namespaced data blob per block.
        let num_expected_blobs = blocks
            .iter()
            .fold(0, |acc, block| acc + block.rollup_transactions().len() + 1);

        // the number of blobs should always be low enough to not cause precision loss
        #[allow(clippy::cast_precision_loss)]
        let number_rollup_blobs = (num_expected_blobs - blocks.len()) as f64;
        metrics::gauge!(
            metric_labels::ROLLUP_BLOBS_PER_CELESTIA_TX,
            number_rollup_blobs
        );

        let mut all_blobs = Vec::with_capacity(num_expected_blobs);
        for (i, block) in blocks.into_iter().enumerate() {
            let mut blobs = assemble_blobs_from_sequencer_block(block).map_err(|source| {
                SubmitSequencerBlocksError::AssembleBlobs {
                    source,
                    index: i,
                }
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
    #[error(
        "failed to construct inclusion proof for the transaction at index `{index}` because its \
         index was outside the tree"
    )]
    ConstructProof { index: usize },
}

fn assemble_blobs_from_sequencer_block(
    block: SequencerBlock,
) -> Result<Vec<Blob>, BlobAssemblyError> {
    let (sequencer_blob, rollup_blobs) = block.into_celestia_blobs();

    // the number of blobs should always be low enough to not cause precision loss
    #[allow(clippy::cast_precision_loss)]
    let rollup_blobs_count = rollup_blobs.len() as f64;
    metrics::gauge!(
        metric_labels::ROLLUP_BLOBS_PER_ASTRIA_BLOCK,
        rollup_blobs_count
    );

    let mut blobs = Vec::with_capacity(rollup_blobs.len() + 1);

    let sequencer_namespace = celestia_namespace_v0_from_cometbft_header(sequencer_blob.header());

    blobs.push(
        Blob::new(
            sequencer_namespace,
            sequencer_blob.into_raw().encode_to_vec(),
        )
        .map_err(BlobAssemblyError::ConstructBlobFromSequencerData)?,
    );
    for (i, blob) in rollup_blobs.into_iter().enumerate() {
        let namespace = celestia_namespace_v0_from_rollup_id(blob.rollup_id());
        blobs.push(
            Blob::new(namespace, blob.into_raw().encode_to_vec()).map_err(|source| {
                BlobAssemblyError::ConstructBlobFromRollupData {
                    source,
                    index: i,
                }
            })?,
        );
    }
    Ok(blobs)
}

/// Attempts to convert the bytes stored in the celestia blobs to [`CelestiaRollupBlob`].
///
/// Drops a blob under the following conditions:
/// + the blob's namespace does not match the provided [`Namespace`]
/// + cannot be decode/convert to [`CelestiaRollupBlob`]
/// + block hash does not match that of [`CcelestiaSequencerBlob`]
/// + the proof, ID, and transactions recorded in the blob cannot be verified against the seuencer
///   blob's `rollup_transaction_root`.
fn convert_and_filter_rollup_blobs(
    blobs: Vec<Blob>,
    namespace: Namespace,
    sequencer_blob: &CelestiaSequencerBlob,
) -> Vec<CelestiaRollupBlob> {
    let mut rollups = Vec::with_capacity(blobs.len());
    for blob in blobs {
        if blob.namespace != namespace {
            warn!("blob does not belong to expected namespace; skipping");
            continue;
        }
        let proto_blob =
            match astria_core::generated::sequencer::v1alpha1::CelestiaRollupBlob::decode(
                &*blob.data,
            ) {
                Err(e) => {
                    warn!(
                        error = &e as &dyn std::error::Error,
                        target = "astria.sequencer.v1alpha.CelestiaRollupBlob",
                        blob.commitment = %Base64Display::new(&blob.commitment.0, &STANDARD),
                        "failed decoding blob as protobuf; skipping"
                    );
                    continue;
                }
                Ok(proto_blob) => proto_blob,
            };
        let rollup_blob = match CelestiaRollupBlob::try_from_raw(proto_blob) {
            Err(e) => {
                warn!(
                    error = &e as &dyn std::error::Error,
                    blob.commitment = %Base64Display::new(&blob.commitment.0, &STANDARD),
                    "failed converting raw protobuf blob to native type; skipping"
                );
                continue;
            }
            Ok(rollup_blob) => rollup_blob,
        };
        if rollup_blob.sequencer_block_hash() != sequencer_blob.block_hash() {
            warn!(
                block_hash.rollup = hex::encode(rollup_blob.sequencer_block_hash()),
                block_hash.sequencer = hex::encode(sequencer_blob.block_hash()),
                "block hash in rollup blob does not match block hash in sequencer blob; dropping \
                 blob"
            );
            continue;
        }
        if !does_rollup_blob_verify_against_sequencer_blob(&rollup_blob, sequencer_blob) {
            warn!(
                "the rollup blob proof applied to its chain ID and transactions did not match the \
                 rollup transactions root in the sequencer blob; dropping the blob"
            );
            continue;
        }
        rollups.push(rollup_blob);
    }
    rollups
}

fn does_rollup_blob_verify_against_sequencer_blob(
    rollup_blob: &CelestiaRollupBlob,
    sequencer_blob: &CelestiaSequencerBlob,
) -> bool {
    rollup_blob
        .proof()
        .audit()
        .with_root(sequencer_blob.rollup_transactions_root())
        .with_leaf_builder()
        .write(&rollup_blob.rollup_id().get())
        .write(&merkle::Tree::from_leaves(rollup_blob.transactions()).root())
        .finish_leaf()
        .perform()
}
