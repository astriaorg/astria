use std::sync::Arc;

use astria_core::{
    generated::sequencerblock::v1alpha1::{
        sequencer_service_server::SequencerService,
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        GetFilteredSequencerBlockRequest,
        GetPendingNonceRequest,
        GetPendingNonceResponse,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    primitive::v1::RollupId,
};
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::{
    error,
    info,
    instrument,
};

use crate::{
    api_state_ext::StateReadExt as _,
    mempool::Mempool,
    state_ext::StateReadExt as _,
};

pub(crate) struct SequencerServer {
    storage: Storage,
    mempool: Mempool,
}

impl SequencerServer {
    pub(crate) fn new(storage: Storage, mempool: Mempool) -> Self {
        Self {
            storage,
            mempool,
        }
    }
}

#[async_trait::async_trait]
impl SequencerService for SequencerServer {
    /// Given a block height, returns the sequencer block at that height.
    #[instrument(skip_all)]
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
    #[instrument(skip_all)]
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
            .iter()
            .map(RollupId::try_from_raw)
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

    #[instrument(skip_all)]
    async fn get_pending_nonce(
        self: Arc<Self>,
        request: Request<GetPendingNonceRequest>,
    ) -> Result<Response<GetPendingNonceResponse>, Status> {
        use astria_core::primitive::v1::Address;

        use crate::accounts::StateReadExt as _;

        let request = request.into_inner();
        let Some(address) = request.address else {
            info!("required field address was not set",);
            return Err(Status::invalid_argument(
                "required field address was not set",
            ));
        };

        let address = Address::try_from_raw(&address).map_err(|e| {
            info!(
                error = %e,
                "failed to parse address from request",
            );
            Status::invalid_argument(format!("invalid address: {e}"))
        })?;
        let nonce = self.mempool.pending_nonce(address.bytes()).await;

        if let Some(nonce) = nonce {
            return Ok(Response::new(GetPendingNonceResponse {
                inner: nonce,
            }));
        }

        // nonce wasn't in mempool, so just look it up from storage
        let snapshot = self.storage.latest_snapshot();
        let nonce = snapshot.get_account_nonce(address).await.map_err(|e| {
            error!(
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                "failed to parse get account nonce from storage",
            );
            Status::internal(format!("failed to get account nonce from storage: {e}"))
        })?;

        Ok(Response::new(GetPendingNonceResponse {
            inner: nonce,
        }))
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        protocol::test_utils::ConfigureSequencerBlock,
        sequencerblock::v1alpha1::SequencerBlock,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        api_state_ext::StateWriteExt as _,
        app::test_utils::get_alice_signing_key,
        state_ext::StateWriteExt,
        test_utils::astria_address,
    };

    fn make_test_sequencer_block(height: u32) -> SequencerBlock {
        ConfigureSequencerBlock {
            height,
            ..Default::default()
        }
        .make()
    }

    #[tokio::test]
    async fn test_get_sequencer_block() {
        let block = make_test_sequencer_block(1);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mempool = Mempool::new();
        let mut state_tx = StateDelta::new(storage.latest_snapshot());
        state_tx.put_block_height(1);
        state_tx.put_sequencer_block(block.clone()).unwrap();
        storage.commit(state_tx).await.unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone(), mempool));
        let request = GetSequencerBlockRequest {
            height: 1,
        };
        let request = Request::new(request);
        let response = server.get_sequencer_block(request).await.unwrap();
        assert_eq!(response.into_inner().header.unwrap().height, 1);
    }

    #[tokio::test]
    async fn get_pending_nonce_in_mempool() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mempool = Mempool::new();

        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        let nonce = 99;
        let tx = Arc::new(crate::app::test_utils::get_mock_tx(nonce));
        mempool.insert(tx, 0).await.unwrap();

        // insert a tx with lower nonce also, but we should get the highest nonce
        let lower_nonce = 98;
        let tx = Arc::new(crate::app::test_utils::get_mock_tx(lower_nonce));
        mempool.insert(tx, 0).await.unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone(), mempool));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, nonce);
    }

    #[tokio::test]
    async fn get_pending_nonce_in_storage() {
        use crate::accounts::StateWriteExt as _;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mempool = Mempool::new();
        let mut state_tx = StateDelta::new(storage.latest_snapshot());
        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        state_tx.put_account_nonce(alice_address, 99).unwrap();
        storage.commit(state_tx).await.unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone(), mempool));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, 99);
    }
}
