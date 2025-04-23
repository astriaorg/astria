use std::sync::Arc;

use astria_core::{
    generated::mempool::v1alpha1::{
        mempool_service_server::MempoolService,
        transaction_status::Status as RawTransactionStatus,
        GetMempoolRequest,
        GetParkedTransactionsRequest,
        GetPendingTransactionsRequest,
        GetRemovalCacheRequest,
        GetTransactionStatusRequest,
        Mempool as MempoolResponse,
        ParkedTransactions,
        PendingTransactions,
        Removal,
        RemovalCache,
        SubmitTransactionRequest,
        SubmitTransactionResponse,
        TransactionStatus as TransactionStatusResponse,
    },
    primitive::v1::TRANSACTION_ID_LEN,
};
use bytes::Bytes;
use sha2::Digest;
use tendermint::abci::{request, Code};
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{mempool::Mempool, service::mempool::handle_check_tx};

use super::sequencer::SequencerServer;

#[async_trait::async_trait]
impl MempoolService for SequencerServer {
    async fn get_mempool(
        self: Arc<Self>,
        _request: Request<GetMempoolRequest>,
    ) -> Result<Response<MempoolResponse>, Status> {
        let pending = {
            let pending_hashes = self.mempool.pending_hashes().await;
            (!pending_hashes.is_empty()).then(|| PendingTransactions {
                inner: pending_hashes
                    .iter()
                    .map(|hash| hash.to_vec().into())
                    .collect(),
            })
        };
        let parked = {
            let parked_hashes = self.mempool.parked_hashes().await;
            (!parked_hashes.is_empty()).then(|| ParkedTransactions {
                inner: parked_hashes
                    .iter()
                    .map(|hash| hash.to_vec().into())
                    .collect(),
            })
        };
        let removed = {
            let removal_cache = self.mempool.removal_cache().await;
            (!removal_cache.is_empty()).then(|| RemovalCache {
                inner: removal_cache
                    .iter()
                    .map(|(hash, removal_reason)| Removal {
                        tx_hash: hash.to_vec().into(),
                        reason: removal_reason.to_string(),
                    })
                    .collect(),
            })
        };
        Ok(Response::new(Mempool {
            pending,
            parked,
            removed,
        }))
    }

    async fn get_parked_transactions(
        self: Arc<Self>,
        _request: Request<GetParkedTransactionsRequest>,
    ) -> Result<Response<ParkedTransactions>, Status> {
        todo!()
    }

    async fn get_pending_transactions(
        self: Arc<Self>,
        _request: Request<GetPendingTransactionsRequest>,
    ) -> Result<Response<PendingTransactions>, Status> {
        todo!()
    }

    async fn get_removal_cache(
        self: Arc<Self>,
        _request: Request<GetRemovalCacheRequest>,
    ) -> Result<Response<RemovalCache>, Status> {
        todo!()
    }

    async fn get_transaction_status(
        self: Arc<Self>,
        request: Request<GetTransactionStatusRequest>,
    ) -> Result<Response<TransactionStatusResponse>, Status> {
        let tx_hash_bytes = request.into_inner().transaction_hash;
        Ok(Response::new(get_transaction_status(
            &self.mempool,
            tx_hash_bytes,
        )
        .await?))
    }

    async fn submit_transaction(
        self: Arc<Self>,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<SubmitTransactionResponse>, Status> {
        let tx_bytes = request.into_inner().transaction;
        let check_tx = request::CheckTx {
            tx: tx_bytes.clone(),
            kind: request::CheckTxKind::New,
        };

        let rsp = handle_check_tx(check_tx, self.storage.latest_snapshot(), &mut self.mempool, self.metrics).await;
        if let Code::Err(_) = rsp.code {
            return Err(Status::internal(format!("Transaction failed CheckTx: {}", rsp.log)));
        };

        let status = get_transaction_status(
            &self.mempool,
            Bytes::from(sha2::Sha256::digest(&tx_bytes).to_vec()),
        )
        .await?;

        Ok(Response::new(SubmitTransactionResponse {
            status: Some(status)
        }))
    }
}

async fn get_transaction_status(
    mempool: &Mempool,
    tx_hash_bytes: Bytes,
) -> Result<TransactionStatusResponse, Status> {
    use crate::mempool::TransactionStatus;
    use astria_core::generated::mempool::v1alpha1::{
        Included as RawIncluded,
        Parked as RawParked,
        Pending as RawPending,
        Removed as RawRemoved,
    };
    let tx_hash: [u8; 32] = tx_hash_bytes.as_ref().try_into().map_err(|_| {
        Status::invalid_argument(format!(
            "Invalid transaction hash contained {} bytes, expected {TRANSACTION_ID_LEN}",
            tx_hash_bytes.len()
        ))
    })?;
    let status = match mempool.transaction_status(&tx_hash).await {
        Some(TransactionStatus::Pending) => Some(RawTransactionStatus::Pending(RawPending {})),
        Some(TransactionStatus::Parked) => Some(RawTransactionStatus::Parked(RawParked {})),
        Some(TransactionStatus::Removed(reason)) => {
            Some(RawTransactionStatus::Removed(RawRemoved {
                reason: reason.to_string(),
            }))
        }
        Some(TransactionStatus::Included(block_number)) => {
            Some(RawTransactionStatus::Included(RawIncluded {
                block_number,
            }))
        }
        None => None,
    };
    Ok(TransactionStatusResponse {
        transaction_hash: tx_hash_bytes,
        status,
    })
}
