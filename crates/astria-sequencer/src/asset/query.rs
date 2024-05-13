use anyhow::Context as _;
use astria_core::{
    primitive::v1::asset,
    protocol::abci::AbciErrorCode,
};
use cnidarium::Storage;
use prost::Message as _;
use tendermint::abci::{
    request,
    response,
};

use crate::{
    asset::state_ext::StateReadExt as _,
    state_ext::StateReadExt,
};

// Retrieve the full asset denomination given the asset ID.
//
// Example:
// `abci-cli query --path=asset/denom/<DENOM_ID>`
pub(crate) async fn denom_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::asset::v1alpha1::DenomResponse;

    // use the latest snapshot, as this is a lookup of id->denom
    let snapshot = storage.latest_snapshot();
    let asset_id = match preprocess_request(&params).await {
        Ok(asset_id) => asset_id,
        Err(err_rsp) => return err_rsp,
    };

    let height = match snapshot.get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed getting block height: {err:?}"),
                ..response::Query::default()
            };
        }
    };

    let denom = match snapshot.get_ibc_asset(asset_id).await {
        Ok(denom) => denom,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed to retrieve denomination: {err:?}"),
                ..response::Query::default()
            };
        }
    };

    let payload = DenomResponse {
        height: height as u64,
        denom: denom.clone(),
    }
    .into_raw()
    .encode_to_vec()
    .into();

    let height = u32::try_from(height).expect("height must fit into a u32");
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height: height.into(),
        ..response::Query::default()
    }
}

async fn preprocess_request(
    params: &[(String, String)],
) -> anyhow::Result<asset::Id, response::Query> {
    let Some(asset_id) = params.iter().find_map(|(k, v)| (k == "id").then_some(v)) else {
        return Err(response::Query {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: AbciErrorCode::INVALID_PARAMETER.to_string(),
            log: "path did not contain path parameter".into(),
            ..response::Query::default()
        });
    };
    let asset_id = hex::decode(asset_id)
        .context("failed decoding hex encoded bytes")
        .and_then(|addr| {
            asset::Id::try_from_slice(&addr).context("failed constructing asset ID from bytes")
        })
        .map_err(|err| response::Query {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: AbciErrorCode::INVALID_PARAMETER.to_string(),
            log: format!("address could not be constructed from provided parameter: {err:?}"),
            ..response::Query::default()
        })?;
    Ok(asset_id)
}
