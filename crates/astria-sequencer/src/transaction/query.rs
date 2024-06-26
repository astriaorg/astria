use astria_core::{
    generated::protocol::transaction::v1alpha1::UnsignedTransaction as RawUnsignedTransaction,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::UnsignedTransaction,
    },
};
use cnidarium::Storage;
use prost::Message as _;
use tendermint::abci::{
    request,
    response,
};

use crate::{
    state_ext::StateReadExt,
    transaction::checks::get_fees_for_transaction,
};

pub(crate) async fn transaction_fee_request(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::transaction::v1alpha1::TransactionFeeResponse;

    let tx = match preprocess_request(&request) {
        Ok(tx) => tx,
        Err(err_rsp) => return err_rsp,
    };

    // use latest snapshot, as this is a query for a transaction fee
    let snapshot = storage.latest_snapshot();
    let height = match snapshot.get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed getting block height: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let fees = match get_fees_for_transaction(&tx, &snapshot).await {
        Ok(fees) => fees,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed calculating fees for provided transaction: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let fees = fees.into_iter().collect();

    let resp = TransactionFeeResponse {
        height,
        fees,
    };

    let payload = resp.into_raw().encode_to_vec().into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: 0.into(),
        key: request.path.into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

fn preprocess_request(request: &request::Query) -> Result<UnsignedTransaction, response::Query> {
    let tx = match RawUnsignedTransaction::decode(&*request.data) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(response::Query {
                code: AbciErrorCode::BAD_REQUEST.into(),
                info: AbciErrorCode::BAD_REQUEST.to_string(),
                log: format!("failed to decode request data to unsigned transaction: {err:#}"),
                ..response::Query::default()
            });
        }
    };

    let tx = match UnsignedTransaction::try_from_raw(tx) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(response::Query {
                code: AbciErrorCode::BAD_REQUEST.into(),
                info: AbciErrorCode::BAD_REQUEST.to_string(),
                log: format!(
                    "failed to convert raw proto unsigned transaction to native unsigned \
                     transaction: {err:#}"
                ),
                ..response::Query::default()
            });
        }
    };

    Ok(tx)
}
