use astria_core::{
    primitive::v1::asset,
    protocol::abci::AbciErrorCode,
};
use astria_eyre::eyre::WrapErr as _;
use hex::FromHex as _;
use prost::Message as _;
use tendermint::abci::{
    request,
    response,
    Code,
};
use tracing::instrument;

use crate::{
    app::StateReadExt as _,
    assets::StateReadExt as _,
    storage::Storage,
};

// Retrieve the full asset denomination given the asset ID.
//
// Example:
// `abci-cli query --path=asset/denom/<DENOM_ID>`
#[instrument(skip_all)]
pub(crate) async fn denom_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::asset::v1::DenomResponse;

    // use the latest snapshot, as this is a lookup of id->denom
    let snapshot = storage.latest_snapshot();
    let asset = match preprocess_request(&params) {
        Ok(asset) => asset,
        Err(err_rsp) => return err_rsp,
    };

    let height = match snapshot.get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed getting block height: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let maybe_denom = match snapshot.map_ibc_to_trace_prefixed_asset(&asset).await {
        Ok(maybe_denom) => maybe_denom,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed to retrieve denomination `{asset}`: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let Some(denom) = maybe_denom else {
        return response::Query {
            code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
            info: AbciErrorCode::VALUE_NOT_FOUND.info(),
            log: format!("failed to retrieve value for denomination ID`{asset}`"),
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

fn preprocess_request(params: &[(String, String)]) -> Result<asset::IbcPrefixed, response::Query> {
    let Some(asset_id) = params.iter().find_map(|(k, v)| (k == "id").then_some(v)) else {
        return Err(response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: "path did not contain asset ID parameter".into(),
            ..response::Query::default()
        });
    };
    let asset = <[u8; 32]>::from_hex(asset_id)
        .wrap_err("failed decoding hex encoded bytes")
        .map(asset::IbcPrefixed::new)
        .map_err(|err| response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: format!("asset ID could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    Ok(asset)
}
