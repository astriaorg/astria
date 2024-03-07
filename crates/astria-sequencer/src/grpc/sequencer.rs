use astria_core::{
    generated::sequencer::v1alpha1::{
        sequencer_service_server::SequencerService,
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        GetFilteredSequencerBlockRequest,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    sequencer::v1alpha1::RollupId,
};
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::instrument;

use crate::{
    api_state_ext::StateReadExt as _,
    state_ext::StateReadExt as _,
};

pub(crate) struct SequencerServer {
    storage: Storage,
}

impl SequencerServer {
    pub(crate) fn new(storage: Storage) -> Self {
        Self {
            storage,
        }
    }
}

#[async_trait::async_trait]
impl SequencerService for SequencerServer {
    /// Given a block height, returns the sequencer block at that height.
    #[instrument(skip_all, fields(height = request.get_ref().height))]
    async fn get_sequencer_block(
        &self,
        request: Request<GetSequencerBlockRequest>,
    ) -> Result<Response<RawSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let block = snapshot
            .get_sequencer_block_by_height(request.height)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to get sequencer block from storage: {e}"))
            })?;

        Ok(Response::new(block.into_raw()))
    }

    /// Given a block height and set of rollup ids, returns a SequencerBlock which
    /// is filtered to contain only the transactions that are relevant to the given rollup.
    #[instrument(skip_all, fields(height = request.get_ref().height))]
    async fn get_filtered_sequencer_block(
        &self,
        request: Request<GetFilteredSequencerBlockRequest>,
    ) -> Result<Response<RawFilteredSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let mut rollup_ids: Vec<RollupId> = vec![];
        for id in request.rollup_ids {
            let Ok(rollup_id) = RollupId::try_from_vec(id) else {
                return Err(Status::invalid_argument(
                    "invalid rollup ID; must be 32 bytes",
                ));
            };
            rollup_ids.push(rollup_id);
        }

        let block_hash = snapshot
            .get_block_hash_by_height(request.height)
            .await
            .map_err(|e| Status::internal(format!("failed to get block hash from storage: {e}")))?;

        let header_parts = snapshot
            .get_sequencer_block_header_by_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get sequencer block header from storage: {e}"
                ))
            })?
            .into_parts();

        let (rollup_transactions_proof, rollup_ids_proof) = snapshot
            .get_block_proofs_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get sequencer block proofs from storage: {e}"
                ))
            })?;

        let all_rollup_ids = snapshot
            .get_rollup_ids_by_block_hash(&block_hash)
            .await
            .map_err(|e| Status::internal(format!("failed to get rollup ids from storage: {e}")))?
            .into_iter()
            .map(RollupId::to_vec)
            .collect::<Vec<_>>();

        let mut rollup_transactions = Vec::with_capacity(rollup_ids.len());
        for rollup_id in rollup_ids {
            let rollup_data = snapshot
                .get_rollup_data(&block_hash, &rollup_id)
                .await
                .map_err(|e| {
                    Status::internal(format!("failed to get rollup data from storage: {e}",))
                })?;
            rollup_transactions.push(rollup_data.into_raw());
        }

        let block = RawFilteredSequencerBlock {
            header: Some(header_parts.cometbft_header.into()),
            rollup_transactions,
            rollup_transactions_root: header_parts.rollup_transactions_root.to_vec(),
            rollup_transactions_proof: rollup_transactions_proof.into(),
            rollup_ids_proof: rollup_ids_proof.into(),
            all_rollup_ids,
        };

        Ok(Response::new(block))
    }
}
