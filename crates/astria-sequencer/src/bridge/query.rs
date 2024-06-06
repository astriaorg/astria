use anyhow::Context as _;
use astria_core::{
    primitive::v1::Address,
    protocol::abci::AbciErrorCode,
};
use cnidarium::Storage;
use prost::Message as _;
use tendermint::abci::{
    request,
    response,
};

use crate::{
    bridge::state_ext::StateReadExt as _,
    state_ext::StateReadExt as _,
};

pub(crate) async fn bridge_account_last_tx_hash_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::bridge::v1alpha1::BridgeAccountLastTxHashResponse;

    let address = match preprocess_request(&params) {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };

    // use latest snapshot, as this is a query for latest tx
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

    let tx_hash = match snapshot
        .get_last_transaction_hash_for_bridge_account(&address)
        .await
    {
        Ok(Some(tx_hash)) => tx_hash,
        Ok(None) => {
            return response::Query {
                code: AbciErrorCode::VALUE_NOT_FOUND.into(),
                info: AbciErrorCode::VALUE_NOT_FOUND.to_string(),
                log: "no transaction hash found for provided address".into(),
                ..response::Query::default()
            };
        }
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed getting balance for provided address: {err:?}"),
                ..response::Query::default()
            };
        }
    };
    let payload = BridgeAccountLastTxHashResponse {
        height,
        tx_hash,
    }
    .into_raw()
    .encode_to_vec()
    .into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: 0.into(),
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

fn preprocess_request(params: &[(String, String)]) -> anyhow::Result<Address, response::Query> {
    let Some(address) = params
        .iter()
        .find_map(|(k, v)| (k == "address").then_some(v))
    else {
        return Err(response::Query {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: AbciErrorCode::INVALID_PARAMETER.to_string(),
            log: "path did not contain address parameter".into(),
            ..response::Query::default()
        });
    };
    let address = hex::decode(address)
        .context("failed decoding hex encoded bytes")
        .and_then(|addr| {
            Address::try_from_slice(&addr).context("failed constructing address from bytes")
        })
        .map_err(|err| response::Query {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: AbciErrorCode::INVALID_PARAMETER.to_string(),
            log: format!("address could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    Ok(address)
}
