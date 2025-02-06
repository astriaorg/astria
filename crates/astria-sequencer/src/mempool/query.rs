use astria_core::{
    primitive::v1::{
        TransactionId,
        TRANSACTION_ID_LEN,
    },
    protocol::{
        abci::AbciErrorCode,
        transaction::v1::TransactionStatusResponse,
    },
    Protobuf as _,
};
use cnidarium::Storage;
use prost::Message;
use tendermint::{
    abci::Code,
    v0_38::abci::{
        request,
        response,
    },
};
use tracing::instrument;

use super::Mempool;
use crate::app::StateReadExt;

#[instrument(skip_all)]
pub(crate) async fn transaction_status_request(
    storage: Storage,
    mempool: Mempool,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let tx_id = match preprocess_request(&params) {
        Ok(tx_id) => tx_id,
        Err(err) => return err,
    };

    let height = match storage.latest_snapshot().get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: "failed getting block height".into(),
                log: format!("{err:?}"),
                ..response::Query::default()
            };
        }
    };

    let height = match height.try_into() {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: "failed converting `u64` block height to type `tendermint::block::Height`"
                    .into(),
                log: format!("{err:?}"),
                ..response::Query::default()
            };
        }
    };

    let (status, reason) = mempool.get_transaction_status(&tx_id).await;

    let log = reason.map_or(String::new(), |r| format!("removal reason: {r}"));

    let payload = TransactionStatusResponse {
        status,
    }
    .to_raw()
    .encode_to_vec()
    .into();

    response::Query {
        code: Code::Ok,
        log,
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

#[instrument(skip_all)]
fn preprocess_request(params: &[(String, String)]) -> Result<TransactionId, response::Query> {
    let Some(tx_id) = params
        .iter()
        .find_map(|(key, value)| (key == "tx_id").then_some(value))
    else {
        return Err(response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: "missing required parameter `tx_id`".into(),
            ..response::Query::default()
        });
    };

    // Transaction hashes may or may not be prefixed with "0x", so it's good UX to support both
    let tx_id = tx_id.trim_start_matches("0x");

    let tx_id: [u8; TRANSACTION_ID_LEN] =
        hex::decode(tx_id)
            .unwrap()
            .try_into()
            .map_err(|err| response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                info: "failed to parse `tx_id` into type `[u8; 32]`".into(),
                log: format!("{err:?}"),
                ..response::Query::default()
            })?;
    Ok(TransactionId::new(tx_id))
}

#[cfg(test)]
mod tests {
    use astria_core::{
        generated::astria::protocol::transaction::v1::TransactionStatusResponse as RawTransactionStatusResponse,
        protocol::transaction::v1::TransactionStatus,
    };
    use telemetry::Metrics as _;

    use super::*;
    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
            },
            test_utils::MockTxBuilder,
            StateWriteExt as _,
        },
        mempool::RemovalReason,
        Metrics,
    };

    #[tokio::test]
    async fn transaction_status_request_pending_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_block_height(0).unwrap();

        storage.commit(state).await.unwrap();

        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(0).build();
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        let request = request::Query {
            data: vec![].into(),
            path: format!("transaction/status/{}", tx.id()),
            height: 0u32.into(),
            prove: false,
        };

        let response_1 = transaction_status_request(
            (*storage).clone(),
            mempool.clone(),
            request.clone(),
            vec![("tx_id".to_string(), tx.id().to_string())],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_1.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_1.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Pending);

        // Check that the transaction hash can be formatted with or without the "0x" prefix
        let response_2 = transaction_status_request(
            (*storage).clone(),
            mempool,
            request,
            vec![("tx_id".to_string(), format!("0x{}", tx.id()))],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_2.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_2.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Pending);
    }

    #[tokio::test]
    async fn transaction_status_request_parked_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_block_height(0).unwrap();

        storage.commit(state).await.unwrap();

        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(1).build(); // The nonce gap (1 vs. 0) will park the transaction
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        let request = request::Query {
            data: vec![].into(),
            path: format!("transaction/status/{}", tx.id()),
            height: 0u32.into(),
            prove: false,
        };

        let response_1 = transaction_status_request(
            (*storage).clone(),
            mempool.clone(),
            request.clone(),
            vec![("tx_id".to_string(), tx.id().to_string())],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_1.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_1.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Parked);

        // Check that the transaction hash can be formatted with or without the "0x" prefix
        let response_2 = transaction_status_request(
            (*storage).clone(),
            mempool,
            request,
            vec![("tx_id".to_string(), format!("0x{}", tx.id()))],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_2.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_2.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Parked);
    }

    #[tokio::test]
    async fn transaction_status_request_unknown_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_block_height(0).unwrap();

        storage.commit(state).await.unwrap();

        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let tx_id = TransactionId::new([0u8; TRANSACTION_ID_LEN]);

        let request = request::Query {
            data: vec![].into(),
            path: format!("transaction/status/{tx_id}"),
            height: 0u32.into(),
            prove: false,
        };

        let response_1 = transaction_status_request(
            (*storage).clone(),
            mempool.clone(),
            request.clone(),
            vec![("tx_id".to_string(), tx_id.to_string())],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_1.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_1.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Unknown);

        // Check that the transaction hash can be formatted with or without the "0x" prefix
        let response_2 = transaction_status_request(
            (*storage).clone(),
            mempool,
            request,
            vec![("tx_id".to_string(), format!("0x{tx_id}"))],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_2.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_2.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::Unknown);
    }

    #[tokio::test]
    async fn transaction_status_request_removal_cache_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_block_height(0).unwrap();

        storage.commit(state).await.unwrap();

        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(0).build();
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        let reason = RemovalReason::FailedPrepareProposal("test".to_string());
        mempool.remove_tx_invalid(tx.clone(), reason.clone()).await;

        let request = request::Query {
            data: vec![].into(),
            path: format!("transaction/status/{}", tx.id()),
            height: 0u32.into(),
            prove: false,
        };

        let response_1 = transaction_status_request(
            (*storage).clone(),
            mempool.clone(),
            request.clone(),
            vec![("tx_id".to_string(), tx.id().to_string())],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_1.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_1.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::RemovalCache);
        assert_eq!(response_1.log, format!("removal reason: {reason}"));

        // Check that the transaction hash can be formatted with or without the "0x" prefix
        let response_2 = transaction_status_request(
            (*storage).clone(),
            mempool,
            request,
            vec![("tx_id".to_string(), format!("0x{}", tx.id()))],
        )
        .await;
        let transaction_status = TransactionStatusResponse::try_from_raw(
            RawTransactionStatusResponse::decode(response_2.value).unwrap(),
        )
        .unwrap();
        assert_eq!(response_2.code, Code::Ok);
        assert_eq!(transaction_status.status, TransactionStatus::RemovalCache);
        assert_eq!(response_2.log, format!("removal reason: {reason}"));
    }
}
