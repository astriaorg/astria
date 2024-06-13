use anyhow::Context as _;
use astria_core::{
    primitive::v1::asset,
    protocol::{
        abci::AbciErrorCode,
        asset::v1alpha1::AllowedFeeAssetIdsResponse,
    },
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
    let asset_id = match preprocess_request(&params) {
        Ok(asset_id) => asset_id,
        Err(err_rsp) => return err_rsp,
    };

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

    let maybe_denom = match snapshot.get_ibc_asset(asset_id).await {
        Ok(maybe_denom) => maybe_denom,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed to retrieve denomination `{asset_id}`: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let Some(denom) = maybe_denom else {
        return response::Query {
            code: AbciErrorCode::VALUE_NOT_FOUND.into(),
            info: AbciErrorCode::VALUE_NOT_FOUND.to_string(),
            log: format!("failed to retrieve value for denomination ID`{asset_id}`"),
            ..response::Query::default()
        };
    };

    let payload = DenomResponse {
        height,
        denom: denom.into(),
    }
    .into_raw()
    .encode_to_vec()
    .into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

fn preprocess_request(params: &[(String, String)]) -> anyhow::Result<asset::Id, response::Query> {
    let Some(asset_id) = params.iter().find_map(|(k, v)| (k == "id").then_some(v)) else {
        return Err(response::Query {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: AbciErrorCode::INVALID_PARAMETER.to_string(),
            log: "path did not contain asset ID parameter".into(),
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
            log: format!("asset ID could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    Ok(asset_id)
}

pub(crate) async fn allowed_fee_asset_ids_request(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    // get last snapshot
    let snapshot = storage.latest_snapshot();

    // get height from snapshot
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

    // get ids from snapshot at height
    let fee_asset_ids = match snapshot.get_allowed_fee_assets().await {
        Ok(fee_asset_ids) => fee_asset_ids,
        Err(err) => {
            return response::Query {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("failed to retrieve allowed fee assets: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let payload = AllowedFeeAssetIdsResponse {
        height,
        fee_asset_ids,
    }
    .into_raw()
    .encode_to_vec()
    .into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}
