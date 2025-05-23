use std::sync::Arc;

use astria_core::{
    generated::{
        mempool::v1::{
            transaction_service_server::TransactionService,
            transaction_status::{
                Executed as RawExecuted,
                Parked as RawParked,
                Pending as RawPending,
                Removed as RawRemoved,
                Status as RawTransactionStatus,
            },
            GetTransactionFeesRequest,
            GetTransactionFeesResponse,
            GetTransactionStatusRequest,
            SubmitTransactionRequest,
            SubmitTransactionResponse,
            TransactionStatus as TransactionStatusResponse,
        },
        protocol::fees::v1::TransactionFee,
    },
    primitive::v1::{
        TransactionId,
        TRANSACTION_ID_LEN,
    },
    protocol::transaction::v1::TransactionBody,
    Protobuf as _,
};
use bytes::Bytes;
use cnidarium::Storage;
use prost::Message as _;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{
    app::StateReadExt as _,
    assets::StateReadExt as _,
    checked_actions::{
        utils::total_fees,
        ActionRef,
    },
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
        let tx_bytes: Bytes = request
            .into_inner()
            .transaction
            .ok_or_else(|| Status::invalid_argument("Transaction is empty"))?
            .encode_to_vec()
            .into();

        let submission_outcome: SubmissionOutcome = check_tx(
            tx_bytes,
            self.storage.latest_snapshot(),
            &self.mempool,
            self.metrics,
        )
        .await
        .try_into()?;

        Ok(Response::new(SubmitTransactionResponse {
            status: Some(TransactionStatusResponse {
                transaction_hash: submission_outcome.tx_id.get().to_vec().into(),
                status: Some(submission_outcome.status),
            }),
            duplicate: submission_outcome.duplicate,
        }))
    }

    async fn get_transaction_fees(
        self: Arc<Self>,
        request: Request<GetTransactionFeesRequest>,
    ) -> Result<Response<GetTransactionFeesResponse>, Status> {
        let tx_body = TransactionBody::try_from_raw(
            request
                .into_inner()
                .transaction_body
                .ok_or_else(|| Status::invalid_argument("transaction is empty"))?,
        )
        .map_err(|err| {
            Status::invalid_argument(format!("failed to decode transaction from raw: {err}"))
        })?;

        let snapshot = self.storage.latest_snapshot();
        let block_height = snapshot
            .get_block_height()
            .await
            .map_err(|err| Status::internal(format!("failed to get block height: {err}")))?;
        let fees_with_ibc_denoms =
            total_fees(tx_body.actions().iter().map(ActionRef::from), &snapshot)
                .await
                .map_err(|err| {
                    Status::internal(format!("failed to get fees for transaction: {err}"))
                })?;
        let mut fees = Vec::with_capacity(fees_with_ibc_denoms.len());
        for (ibc_denom, value) in fees_with_ibc_denoms {
            let trace_denom = match snapshot.map_ibc_to_trace_prefixed_asset(&ibc_denom).await {
                Ok(Some(trace_denom)) => trace_denom,
                Ok(None) => {
                    return Err(Status::internal(format!(
                        "failed mapping ibc denom to trace denom: {ibc_denom}; asset does not \
                         exist in state"
                    )));
                }
                Err(err) => {
                    return Err(Status::internal(format!(
                        "failed mapping ibc denom to trace denom: {err:#}"
                    )));
                }
            };
            fees.push(TransactionFee {
                asset: trace_denom.to_string(),
                fee: Some(value.into()),
            });
        }
        Ok(Response::new(GetTransactionFeesResponse {
            block_height,
            fees,
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
    let tx_id = TransactionId::new(tx_hash);
    let status = match mempool.transaction_status(&tx_id).await {
        Some(TransactionStatus::Pending) => Some(RawTransactionStatus::Pending(RawPending {})),
        Some(TransactionStatus::Parked) => Some(RawTransactionStatus::Parked(RawParked {})),
        Some(TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height,
            result,
        })) => Some(RawTransactionStatus::Executed(RawExecuted {
            height,
            result: Some(result.to_raw()),
        })),
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
    tx_id: TransactionId,
    status: RawTransactionStatus,
    duplicate: bool,
}

impl TryFrom<CheckTxOutcome> for SubmissionOutcome {
    type Error = Status;

    fn try_from(value: CheckTxOutcome) -> Result<SubmissionOutcome, Self::Error> {
        match value {
            CheckTxOutcome::AddedToPending(tx_id) => Ok(SubmissionOutcome {
                tx_id,
                status: RawTransactionStatus::Pending(RawPending {}),
                duplicate: false,
            }),
            CheckTxOutcome::AddedToParked(tx_id) => Ok(SubmissionOutcome {
                tx_id,
                status: RawTransactionStatus::Parked(RawParked {}),
                duplicate: false,
            }),
            CheckTxOutcome::AlreadyInPending(tx_id) => Ok(SubmissionOutcome {
                tx_id,
                status: RawTransactionStatus::Pending(RawPending {}),
                duplicate: true,
            }),
            CheckTxOutcome::AlreadyInParked(tx_id) => Ok(SubmissionOutcome {
                tx_id,
                status: RawTransactionStatus::Parked(RawParked {}),
                duplicate: true,
            }),
            CheckTxOutcome::FailedChecks(err) => Err(err.into()),
            CheckTxOutcome::FailedInsertion(err) => Err(err.into()),
            CheckTxOutcome::InternalError(source) => {
                Err(Status::internal(format!("internal error: {source}")))
            }
            CheckTxOutcome::RemovedFromMempool {
                reason, ..
            } => Err(Status::not_found(format!(
                "transaction has been removed from the app-side mempool: {reason}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use astria_core::{
        generated::protocol::transaction::v1::Transaction as RawTransaction,
        primitive::v1::RollupId,
        protocol::transaction::v1::{
            action::{
                RollupDataSubmission,
                Transfer,
            },
            TransactionBody,
        },
        Protobuf as _,
    };
    use cnidarium::StateDelta;
    use prost::Message as _;
    use tendermint::abci::types::ExecTxResult;

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        assets::StateWriteExt,
        checked_transaction::CheckedTransaction,
        fees::{
            FeeHandler as _,
            StateReadExt as _,
            StateWriteExt as _,
        },
        mempool::RemovalReason,
        test_utils::{
            denom_0,
            denom_1,
            Fixture,
            ALICE,
            ALICE_ADDRESS,
            ALICE_ADDRESS_BYTES,
        },
    };

    fn new_server(fixture: &Fixture) -> Arc<Server> {
        Arc::new(Server::new(
            fixture.storage(),
            fixture.mempool(),
            fixture.metrics(),
        ))
    }

    async fn new_tx(fixture: &Fixture, nonce: u32) -> Arc<CheckedTransaction> {
        fixture
            .checked_tx_builder()
            .with_nonce(nonce)
            .with_signer(ALICE.clone())
            .build()
            .await
    }

    #[tokio::test]
    async fn transaction_status_pending_works_as_expected() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let server = new_server(&fixture);

        let nonce = 1;
        let tx = new_tx(&fixture, nonce).await;
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        // Should be inserted into Pending
        mempool
            .insert(tx, nonce, &HashMap::new(), HashMap::new())
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
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let server = new_server(&fixture);

        let nonce = 1;
        let tx = new_tx(&fixture, nonce).await;
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
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let server = new_server(&fixture);

        let nonce = 1;
        let tx = new_tx(&fixture, nonce).await;
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
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, nonce.saturating_add(1))
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = new_server(&fixture);

        let tx = new_tx(&fixture, nonce).await;
        let tx_hash_bytes: Bytes = tx.id().get().to_vec().into();
        mempool
            .insert(tx.clone(), nonce, &HashMap::default(), HashMap::default())
            .await
            .unwrap();
        let height = 100;
        let mut execution_results = HashMap::new();
        let exec_tx_result = ExecTxResult {
            log: "ethan_was_here".to_string(),
            ..ExecTxResult::default()
        };
        execution_results.insert(*tx.id(), Arc::new(exec_tx_result.clone()));
        mempool
            .run_maintenance(&storage.latest_snapshot(), false, execution_results, height)
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
                height,
                result: Some(exec_tx_result.into_raw()),
            }))
        );
    }

    #[tokio::test]
    async fn transaction_status_fails_if_invalid_address() {
        let fixture = Fixture::default_initialized().await;
        let server = new_server(&fixture);

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
        let fixture = Fixture::default_initialized().await;
        let server = new_server(&fixture);

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
        let fixture = Fixture::default_initialized().await;
        let storage = fixture.storage();

        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, nonce)
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = new_server(&fixture);

        let checked_tx = new_tx(&fixture, nonce).await;
        let raw_tx = RawTransaction::decode(checked_tx.encoded_bytes().clone()).unwrap();

        let req = SubmitTransactionRequest {
            transaction: Some(raw_tx),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            rsp.status.clone().unwrap().transaction_hash,
            Bytes::from(checked_tx.id().get().to_vec())
        );
        assert_eq!(
            rsp.status.unwrap().status.unwrap(),
            RawTransactionStatus::Pending(RawPending {})
        );
        assert!(!rsp.duplicate);
    }

    #[tokio::test]
    async fn submit_transaction_returns_duplicate_if_already_in_mempool() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let storage = fixture.storage();

        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let nonce = 1u32;
        state_delta
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, nonce)
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = new_server(&fixture);

        let checked_tx = new_tx(&fixture, nonce).await;
        let raw_tx = RawTransaction::decode(checked_tx.encoded_bytes().clone()).unwrap();

        mempool
            .insert(
                checked_tx.clone(),
                nonce,
                &HashMap::default(),
                HashMap::default(),
            )
            .await
            .unwrap();

        let req = SubmitTransactionRequest {
            transaction: Some(raw_tx),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            rsp.status.clone().unwrap().transaction_hash,
            Bytes::from(checked_tx.id().get().to_vec())
        );
        assert_eq!(
            rsp.status.unwrap().status.unwrap(),
            RawTransactionStatus::Pending(RawPending {})
        );
        assert!(rsp.duplicate);
    }

    #[tokio::test]
    async fn submit_transaction_fails_if_check_tx_fails() {
        let fixture = Fixture::default_initialized().await;
        let storage = fixture.storage();

        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        let tx_nonce = 1_u32;
        let account_nonce = tx_nonce.checked_add(1).unwrap();
        state_delta
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, account_nonce)
            .unwrap();
        storage.commit(state_delta).await.unwrap();

        let server = new_server(&fixture);

        let checked_tx = new_tx(&fixture, tx_nonce).await;
        let raw_tx = RawTransaction::decode(checked_tx.encoded_bytes().clone()).unwrap();

        let req = SubmitTransactionRequest {
            transaction: Some(raw_tx),
        };
        let rsp = server
            .submit_transaction(Request::new(req))
            .await
            .unwrap_err();
        assert_eq!(rsp.code(), tonic::Code::InvalidArgument);
        assert_eq!(
            rsp.message(),
            "transaction nonce already used; current nonce `2`, transaction nonce `1`"
        );
    }

    #[tokio::test]
    async fn get_transaction_fees_fails_if_transaction_missing() {
        let fixture = Fixture::default_initialized().await;

        let server = new_server(&fixture);

        let req = GetTransactionFeesRequest {
            transaction_body: None,
        };
        let rsp = server
            .get_transaction_fees(Request::new(req))
            .await
            .unwrap_err();
        assert_eq!(rsp.code(), tonic::Code::InvalidArgument);
        assert_eq!(rsp.message(), "transaction is empty".to_string());
    }

    #[tokio::test]
    async fn get_transaction_fees_works_as_expected() {
        let action_a = Transfer {
            to: *ALICE_ADDRESS,
            amount: 100,
            asset: denom_0(),
            fee_asset: denom_0(),
        };
        let action_b = RollupDataSubmission {
            data: vec![1, 2, 3].into(),
            rollup_id: RollupId::from_unhashed_bytes(b"rollupid"),
            fee_asset: denom_1(),
        };
        let chain_id = "test";

        let body = TransactionBody::builder()
            .actions(vec![action_a.clone().into(), action_b.clone().into()])
            .chain_id(chain_id)
            .nonce(0)
            .try_build()
            .unwrap();

        let mut fixture = Fixture::default_initialized().await;
        let mut state = fixture.app.new_state_delta();
        state.put_allowed_fee_asset(&denom_1()).unwrap();
        state
            .put_ibc_asset(denom_1().as_trace_prefixed().unwrap().clone())
            .unwrap();
        fixture.app.apply_and_commit(state, fixture.storage()).await;

        let server = new_server(&fixture);

        let transfer_fees = fixture
            .state()
            .get_fees::<Transfer>()
            .await
            .unwrap()
            .unwrap();
        let rollup_data_submission_fees = fixture
            .state()
            .get_fees::<RollupDataSubmission>()
            .await
            .unwrap()
            .unwrap();

        let request = Request::new(GetTransactionFeesRequest {
            transaction_body: Some(body.into_raw()),
        });

        let mut rsp = server
            .get_transaction_fees(request)
            .await
            .unwrap()
            .into_inner();

        let mut expected = GetTransactionFeesResponse {
            block_height: fixture.block_height().await.value(),
            fees: vec![
                TransactionFee {
                    asset: denom_0().to_string(),
                    fee: Some(
                        (transfer_fees.base()
                            + transfer_fees.multiplier() * action_a.variable_component())
                        .into(),
                    ),
                },
                TransactionFee {
                    asset: denom_1().to_string(),
                    fee: Some(
                        (rollup_data_submission_fees.base()
                            + rollup_data_submission_fees.multiplier()
                                * action_b.variable_component())
                        .into(),
                    ),
                },
            ],
        };
        rsp.fees
            .sort_by(|a, b| u128::from(a.fee.unwrap()).cmp(&u128::from(b.fee.unwrap())));
        expected
            .fees
            .sort_by(|a, b| u128::from(a.fee.unwrap()).cmp(&u128::from(b.fee.unwrap())));

        assert_eq!(rsp, expected);
    }
}
