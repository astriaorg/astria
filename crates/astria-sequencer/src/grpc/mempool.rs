use std::sync::Arc;

use astria_core::{
    generated::mempool::v1::{
        transaction_service_server::TransactionService,
        transaction_status::{
            Executed as RawExecuted,
            Parked as RawParked,
            Pending as RawPending,
            Removed as RawRemoved,
            Status as RawTransactionStatus,
        },
        GetTransactionStatusRequest,
        SubmitTransactionRequest,
        SubmitTransactionResponse,
        TransactionStatus as TransactionStatusResponse,
    },
    primitive::v1::TRANSACTION_ID_LEN,
    protocol::transaction::v1::Transaction,
};
use bytes::Bytes;
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{
    mempool::{
        Mempool,
        RemovalReason,
        TransactionStatus,
    },
    service::mempool::{
        check_tx,
        CheckTxOutcome,
    },
    Metrics,
};

pub(crate) struct Server {
    storage: Storage,
    mempool: Mempool,
    metrics: &'static Metrics,
}

impl Server {
    pub(crate) fn new(storage: Storage, mempool: Mempool, metrics: &'static Metrics) -> Self {
        Self {
            storage,
            mempool,
            metrics,
        }
    }
}

#[async_trait::async_trait]
impl TransactionService for Server {
    async fn get_transaction_status(
        self: Arc<Self>,
        request: Request<GetTransactionStatusRequest>,
    ) -> Result<Response<TransactionStatusResponse>, Status> {
        let tx_hash_bytes = request.into_inner().transaction_hash;
        Ok(Response::new(
            get_transaction_status(&self.mempool, tx_hash_bytes).await?,
        ))
    }

    async fn submit_transaction(
        self: Arc<Self>,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<SubmitTransactionResponse>, Status> {
        let tx: Transaction = request
            .into_inner()
            .transaction
            .ok_or_else(|| Status::invalid_argument("Transaction is empty"))?
            .try_into()
            .map_err(|err| {
                Status::invalid_argument(format!("Raw transaction is invalid: {err}"))
            })?;

        let tx_hash_bytes = tx.id().get().to_vec().into();

        let submission_outcome: SubmissionOutcome = check_tx(
            tx,
            self.storage.latest_snapshot(),
            &self.mempool,
            self.metrics,
        )
        .await
        .try_into()?;

        Ok(Response::new(SubmitTransactionResponse {
            status: Some(TransactionStatusResponse {
                transaction_hash: tx_hash_bytes,
                status: Some(submission_outcome.status),
            }),
            duplicate: submission_outcome.duplicate,
        }))
    }
}

async fn get_transaction_status(
    mempool: &Mempool,
    tx_hash_bytes: Bytes,
) -> Result<TransactionStatusResponse, Status> {
    let tx_hash: [u8; 32] = tx_hash_bytes.as_ref().try_into().map_err(|_| {
        Status::invalid_argument(format!(
            "Invalid transaction hash contained {} bytes, expected {TRANSACTION_ID_LEN}",
            tx_hash_bytes.len()
        ))
    })?;
    let status = match mempool.transaction_status(&tx_hash).await {
        Some(TransactionStatus::Pending) => Some(RawTransactionStatus::Pending(RawPending {})),
        Some(TransactionStatus::Parked) => Some(RawTransactionStatus::Parked(RawParked {})),
        Some(TransactionStatus::Removed(RemovalReason::IncludedInBlock(height))) => {
            Some(RawTransactionStatus::Executed(RawExecuted {
                height,
            }))
        }
        Some(TransactionStatus::Removed(reason)) => {
            Some(RawTransactionStatus::Removed(RawRemoved {
                reason: reason.to_string(),
            }))
        }
        None => None,
    };
    Ok(TransactionStatusResponse {
        transaction_hash: tx_hash_bytes,
        status,
    })
}

struct SubmissionOutcome {
    status: RawTransactionStatus,
    duplicate: bool,
}

impl TryFrom<CheckTxOutcome> for SubmissionOutcome {
    type Error = Status;

    fn try_from(value: CheckTxOutcome) -> Result<SubmissionOutcome, Self::Error> {
        match value {
            CheckTxOutcome::AddedToPending => Ok(SubmissionOutcome {
                status: RawTransactionStatus::Pending(RawPending {}),
                duplicate: false,
            }),
            CheckTxOutcome::AddedToParked => Ok(SubmissionOutcome {
                status: RawTransactionStatus::Parked(RawParked {}),
                duplicate: false,
            }),
            CheckTxOutcome::AlreadyInPending => Ok(SubmissionOutcome {
                status: RawTransactionStatus::Pending(RawPending {}),
                duplicate: true,
            }),
            CheckTxOutcome::AlreadyInParked => Ok(SubmissionOutcome {
                status: RawTransactionStatus::Parked(RawParked {}),
                duplicate: true,
            }),
            CheckTxOutcome::FailedStatelessChecks {
                source,
            } => Err(tonic::Status::invalid_argument(format!(
                "transaction failed stateless checks: {source}"
            ))),
            CheckTxOutcome::FailedInsertion(err) => Err(err.into()),
            CheckTxOutcome::InternalError {
                source,
            } => Err(tonic::Status::internal(format!("internal error: {source}"))),
            CheckTxOutcome::InvalidChainId {
                expected,
                actual,
            } => Err(tonic::Status::invalid_argument(format!(
                "invalid chain id; expected: {expected}, got: {actual}"
            ))),
            CheckTxOutcome::InvalidTransactionProtobuf {
                source,
            } => Err(tonic::Status::invalid_argument(format!(
                "invalid transaction protobuf: {source}"
            ))),
            CheckTxOutcome::InvalidTransactionBytes {
                name,
                source,
            } => Err(tonic::Status::invalid_argument(format!(
                "failed decoding bytes as a protobuf {name}: {source}"
            ))),
            CheckTxOutcome::RemovedFromMempool(removal_reason) => Err(tonic::Status::not_found(
                format!("transaction has been removed from the app-side mempool: {removal_reason}"),
            )),
            CheckTxOutcome::TransactionTooLarge {
                max_size,
                actual_size,
            } => Err(tonic::Status::invalid_argument(format!(
                "transaction size too large; allowed: {max_size} bytes, got {actual_size}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{
        HashMap,
        HashSet,
    };

    use astria_core::{
        primitive::v1::{
            RollupId,
            ROLLUP_ID_LEN,
        },
        protocol::{
            fees::v1::FeeComponents,
            transaction::v1::{
                action::RollupDataSubmission,
                Transaction,
                TransactionBodyBuilder,
            },
        },
        Protobuf as _,
    };
    use cnidarium::StateDelta;
    use telemetry::Metrics as _;

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        address::StateWriteExt as _,
        app::{
            test_utils::get_alice_signing_key,
            StateWriteExt as _,
        },
        benchmark_and_test_utils::nria,
        fees::StateWriteExt as _,
        mempool::RemovalReason,
        Metrics,
    };

    const TEST_CHAIN_ID: &str = "test_chain_id";

    fn make_transaction(nonce: u32) -> Transaction {
        TransactionBodyBuilder::new()
            .actions(vec![RollupDataSubmission {
                rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
                data: vec![0; 100].into(),
                fee_asset: nria().into(),
            }
            .into()])
            .nonce(nonce)
            .chain_id(TEST_CHAIN_ID.to_string())
            .try_build()
            .unwrap()
            .sign(&get_alice_signing_key())
    }

    #[tokio::test]
    async fn transaction_status_pending_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let nonce = 1;
        let tx = Arc::new(make_transaction(nonce));
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        // Should be inserted into Pending
        mempool
            .insert(tx.clone(), nonce, &HashMap::default(), HashMap::default())
            .await
            .unwrap();

        let req = GetTransactionStatusRequest {
            transaction_hash: tx_hash_bytes.clone(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.transaction_hash, tx_hash_bytes);
        assert_eq!(
            rsp.status,
            Some(RawTransactionStatus::Pending(RawPending {}))
        );
    }

    #[tokio::test]
    async fn transaction_status_parked_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let nonce = 1;
        let tx = Arc::new(make_transaction(nonce));
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        // Should be inserted into Parked due to nonce gap
        mempool
            .insert(
                tx.clone(),
                nonce.saturating_sub(1),
                &HashMap::default(),
                HashMap::default(),
            )
            .await
            .unwrap();

        let req = GetTransactionStatusRequest {
            transaction_hash: tx_hash_bytes.clone(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.transaction_hash, tx_hash_bytes);
        assert_eq!(rsp.status, Some(RawTransactionStatus::Parked(RawParked {})));
    }

    #[tokio::test]
    async fn transaction_status_removed_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let nonce = 1;
        let tx = Arc::new(make_transaction(nonce));
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        mempool
            .insert(tx.clone(), nonce, &HashMap::default(), HashMap::default())
            .await
            .unwrap();

        let removal_reason = RemovalReason::FailedPrepareProposal("failure reason".to_string());
        mempool
            .remove_tx_invalid(tx.clone(), removal_reason.clone())
            .await;

        let req = GetTransactionStatusRequest {
            transaction_hash: tx_hash_bytes.clone(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.transaction_hash, tx_hash_bytes);
        assert_eq!(
            rsp.status,
            Some(RawTransactionStatus::Removed(RawRemoved {
                reason: removal_reason.to_string()
            }))
        );
    }

    #[tokio::test]
    async fn transaction_status_included_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(
                &get_alice_signing_key().address_bytes(),
                nonce.saturating_add(1),
            )
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let tx = Arc::new(make_transaction(nonce));
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        mempool
            .insert(tx.clone(), nonce, &HashMap::default(), HashMap::default())
            .await
            .unwrap();
        let height = 100;
        let mut included_txs = HashSet::new();
        included_txs.insert(tx.id().get());
        mempool
            .run_maintenance(&storage.latest_snapshot(), false, included_txs, height)
            .await;

        let req = GetTransactionStatusRequest {
            transaction_hash: tx_hash_bytes.clone(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.transaction_hash, tx_hash_bytes);
        assert_eq!(
            rsp.status,
            Some(RawTransactionStatus::Executed(RawExecuted {
                height
            }))
        );
    }

    #[tokio::test]
    async fn transaction_status_fails_if_invalid_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let wrong_hash_len = TRANSACTION_ID_LEN.saturating_sub(10);
        let req = GetTransactionStatusRequest {
            transaction_hash: vec![0; wrong_hash_len].into(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap_err();
        assert_eq!(rsp.code(), tonic::Code::InvalidArgument);
        assert_eq!(
            rsp.message(),
            format!(
                "Invalid transaction hash contained {wrong_hash_len} bytes, expected \
                 {TRANSACTION_ID_LEN}",
            )
        );
    }

    #[tokio::test]
    async fn transaction_status_returns_none_if_not_found() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let tx_hash_bytes: Bytes = vec![0; TRANSACTION_ID_LEN].into();
        let req = GetTransactionStatusRequest {
            transaction_hash: tx_hash_bytes.clone(),
        };
        let rsp = server
            .get_transaction_status(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.transaction_hash, tx_hash_bytes);
        assert_eq!(rsp.status, None);
    }

    #[tokio::test]
    async fn submit_transaction_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(&get_alice_signing_key().address_bytes(), nonce)
            .unwrap();
        state_delta
            .put_fees(FeeComponents::<RollupDataSubmission>::new(0, 0))
            .unwrap();
        state_delta.put_base_prefix("astria".to_string()).unwrap();
        state_delta
            .put_chain_id_and_revision_number(
                tendermint::chain::Id::try_from(TEST_CHAIN_ID.to_string()).unwrap(),
            )
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let tx = Arc::new(make_transaction(nonce));

        let req = SubmitTransactionRequest {
            transaction: Some(tx.to_raw()),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            rsp.status.clone().unwrap().transaction_hash,
            Bytes::from(tx.id().get().to_vec())
        );
        assert_eq!(
            rsp.status.unwrap().status.unwrap(),
            RawTransactionStatus::Pending(RawPending {})
        );
        assert!(!rsp.duplicate);
    }

    #[tokio::test]
    async fn submit_transaction_returns_duplicate_if_already_in_mempool() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(&get_alice_signing_key().address_bytes(), nonce)
            .unwrap();
        state_delta
            .put_fees(FeeComponents::<RollupDataSubmission>::new(0, 0))
            .unwrap();
        state_delta.put_base_prefix("astria".to_string()).unwrap();
        state_delta
            .put_chain_id_and_revision_number(
                tendermint::chain::Id::try_from(TEST_CHAIN_ID.to_string()).unwrap(),
            )
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let tx = Arc::new(make_transaction(nonce));
        mempool
            .insert(tx.clone(), nonce, &HashMap::default(), HashMap::default())
            .await
            .unwrap();

        let req = SubmitTransactionRequest {
            transaction: Some(tx.to_raw()),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            rsp.status.clone().unwrap().transaction_hash,
            Bytes::from(tx.id().get().to_vec())
        );
        assert_eq!(
            rsp.status.unwrap().status.unwrap(),
            RawTransactionStatus::Pending(RawPending {})
        );
        assert!(rsp.duplicate);
    }

    #[tokio::test]
    async fn submit_transaction_fails_if_check_tx_fails() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(
                &get_alice_signing_key().address_bytes(),
                nonce.saturating_add(1),
            )
            .unwrap();
        state_delta
            .put_fees(FeeComponents::<RollupDataSubmission>::new(0, 0))
            .unwrap();
        state_delta.put_base_prefix("astria".to_string()).unwrap();
        state_delta
            .put_chain_id_and_revision_number(
                tendermint::chain::Id::try_from(TEST_CHAIN_ID.to_string()).unwrap(),
            )
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = Arc::new(Server::new(storage.clone(), mempool.clone(), metrics));

        let tx = Arc::new(make_transaction(nonce));

        let req = SubmitTransactionRequest {
            transaction: Some(tx.to_raw()),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap_err();
        assert_eq!(rsp.code(), tonic::Code::InvalidArgument);
        assert_eq!(
            rsp.message(),
            "given nonce has already been used previously".to_string()
        );
    }
}
