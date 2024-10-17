use std::sync::Arc;

use astria_core::{
    generated::sequencerblock::v1::{
        sequencer_service_server::SequencerService,
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        GetFilteredSequencerBlockRequest,
        GetPendingNonceRequest,
        GetPendingNonceResponse,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    primitive::v1::RollupId,
    Protobuf,
};
use bytes::Bytes;
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
    app::StateReadExt as _,
    grpc::StateReadExt as _,
    mempool::Mempool,
    storage::Storage,
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
            .map(RollupId::try_from_raw_ref)
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

        let rollup_transactions_proof = snapshot
            .get_rollup_transactions_proof_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get rollup transactions proof from storage: {e}"
                ))
            })?;

        let rollup_ids_proof = snapshot
            .get_rollup_ids_proof_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to get rollup ids proof from storage: {e}"))
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

        let all_rollup_ids = all_rollup_ids.into_iter().map(RollupId::into_raw).collect();

        let block = RawFilteredSequencerBlock {
            block_hash: Bytes::copy_from_slice(&block_hash),
            header: Some(header.into_raw()),
            rollup_transactions,
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
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
        let nonce = self.mempool.pending_nonce(address.as_bytes()).await;

        if let Some(nonce) = nonce {
            return Ok(Response::new(GetPendingNonceResponse {
                inner: nonce,
            }));
        }

        // nonce wasn't in mempool, so just look it up from storage
        let snapshot = self.storage.latest_snapshot();
        let nonce = snapshot.get_account_nonce(&address).await.map_err(|e| {
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
mod tests {
    use astria_core::{
        protocol::test_utils::ConfigureSequencerBlock,
        sequencerblock::v1::SequencerBlock,
    };
    use telemetry::Metrics;

    use super::*;
    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
            },
            test_utils::get_alice_signing_key,
            StateWriteExt as _,
        },
        benchmark_and_test_utils::astria_address,
        grpc::StateWriteExt as _,
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        state_delta.put_block_height(1).unwrap();
        state_delta.put_sequencer_block(block).unwrap();
        storage.commit(state_delta).await.unwrap();

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
        let storage = Storage::new_temp().await;
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        // insert a transaction with a nonce gap
        let gapped_nonce = 99;
        let tx = crate::app::test_utils::MockTxBuilder::new()
            .nonce(gapped_nonce)
            .build();
        mempool
            .insert(tx, 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        // insert a transaction at the current nonce
        let account_nonce = 0;
        let tx = crate::app::test_utils::MockTxBuilder::new()
            .nonce(account_nonce)
            .build();

        mempool
            .insert(tx, 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        // insert a transactions one above account nonce (not gapped)
        let sequential_nonce = 1;
        let tx: Arc<astria_core::protocol::transaction::v1::Transaction> =
            crate::app::test_utils::MockTxBuilder::new()
                .nonce(sequential_nonce)
                .build();
        mempool
            .insert(tx, 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone(), mempool));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, sequential_nonce);
    }

    #[tokio::test]
    async fn get_pending_nonce_in_storage() {
        use crate::accounts::StateWriteExt as _;

        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        state_delta.put_account_nonce(&alice_address, 99).unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = Arc::new(SequencerServer::new(storage.clone(), mempool));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, 99);
    }
}
