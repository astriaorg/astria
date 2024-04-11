use std::sync::Arc;

use astria_core::{
    generated::sequencerblock::v1alpha1::{
        sequencer_service_server::SequencerService,
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        GetFilteredSequencerBlockRequest,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    sequencer::v1::RollupId,
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
        self: Arc<Self>,
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
        self: Arc<Self>,
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

        let rollup_ids = request
            .rollup_ids
            .into_iter()
            .map(RollupId::try_from_vec)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Status::invalid_argument(format!("invalid rollup ID: {e}")))?;

        let block_hash = snapshot
            .get_block_hash_by_height(request.height)
            .await
            .map_err(|e| Status::internal(format!("failed to get block hash from storage: {e}")))?;

        let header = snapshot
            .get_sequencer_block_header_by_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get sequencer block header from storage: {e}"
                ))
            })?;

        let (rollup_transactions_proof, rollup_ids_proof) = snapshot
            .get_block_proofs_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get sequencer block proofs from storage: {e}"
                ))
            })?;

        let mut all_rollup_ids = snapshot
            .get_rollup_ids_by_block_hash(&block_hash)
            .await
            .map_err(|e| Status::internal(format!("failed to get rollup ids from storage: {e}")))?;
        all_rollup_ids.sort_unstable();

        // Filter out the Rollup Ids requested which have no data before grabbing
        // so as to not error because the block had no data for the requested rollup
        let rollup_ids: Vec<RollupId> = rollup_ids
            .into_iter()
            .filter(|id| all_rollup_ids.binary_search(id).is_ok())
            .collect();
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

        let all_rollup_ids = all_rollup_ids.into_iter().map(RollupId::to_vec).collect();

        let block = RawFilteredSequencerBlock {
            block_hash: block_hash.to_vec(),
            header: Some(header.into_raw()),
            rollup_transactions,
            rollup_transactions_proof: rollup_transactions_proof.into(),
            rollup_ids_proof: rollup_ids_proof.into(),
            all_rollup_ids,
        };

        Ok(Response::new(block))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use astria_core::sequencerblock::v1alpha1::SequencerBlock;
    use cnidarium::StateDelta;
    use sha2::{
        Digest as _,
        Sha256,
    };
    use tendermint::{
        account,
        block::{
            header::Version,
            Header,
            Height,
        },
        AppHash,
        Hash,
        Time,
    };

    use super::*;
    use crate::{
        api_state_ext::StateWriteExt as _,
        state_ext::StateWriteExt,
    };

    fn make_test_sequencer_block(height: u32) -> SequencerBlock {
        let mut header = Header {
            app_hash: AppHash::try_from(vec![]).unwrap(),
            chain_id: "test".to_string().try_into().unwrap(),
            consensus_hash: Hash::default(),
            data_hash: Some(Hash::try_from([0u8; 32].to_vec()).unwrap()),
            evidence_hash: Some(Hash::default()),
            height: Height::default(),
            last_block_id: None,
            last_commit_hash: Some(Hash::default()),
            last_results_hash: Some(Hash::default()),
            next_validators_hash: Hash::default(),
            proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
            time: Time::now(),
            validators_hash: Hash::default(),
            version: Version {
                app: 0,
                block: 0,
            },
        };

        let empty_hash = merkle::Tree::from_leaves(Vec::<Vec<u8>>::new()).root();
        let block_data = vec![empty_hash.to_vec(), empty_hash.to_vec()];
        let data_hash = merkle::Tree::from_leaves(block_data.iter().map(Sha256::digest)).root();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());
        header.height = height.into();
        SequencerBlock::try_from_cometbft_header_and_data(header, block_data, HashMap::new())
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_sequencer_block() {
        let block = make_test_sequencer_block(1);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state_tx = StateDelta::new(storage.latest_snapshot());
        state_tx.put_block_height(1);
        state_tx.put_sequencer_block(block.clone()).unwrap();
        storage.commit(state_tx).await.unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone()));
        let request = GetSequencerBlockRequest {
            height: 1,
        };
        let request = Request::new(request);
        let response = server.get_sequencer_block(request).await.unwrap();
        assert_eq!(response.into_inner().header.unwrap().height, 1);
    }
}
