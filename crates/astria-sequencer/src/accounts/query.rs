use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::{
        abci::AbciErrorCode,
        account::v1alpha1::AssetBalance,
    },
};
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    Snapshot,
    StateRead,
    Storage,
};
use futures::TryStreamExt as _;
use prost::Message as _;
use tendermint::{
    abci::{
        request,
        response,
        Code,
    },
    block::Height,
};
use tracing::instrument;

use crate::{
    accounts::StateReadExt as _,
    assets::StateReadExt as _,
    state_ext::StateReadExt as _,
};

async fn ibc_to_trace<S: StateRead>(
    state: S,
    asset: asset::IbcPrefixed,
) -> Result<asset::TracePrefixed> {
    state
        .map_ibc_to_trace_prefixed_asset(asset)
        .await
        .context("failed to get ibc asset denom")?
        .ok_or_eyre("asset not found when user has balance of it; this is a bug")
}

#[instrument(skip_all, fields(%address))]
async fn get_trace_prefixed_account_balances<S: StateRead>(
    state: &S,
    address: Address,
) -> Result<Vec<AssetBalance>> {
    let stream = state
        .account_asset_balances(address)
        .map_ok(|asset_balance| async move {
            let native_asset = state
                .get_native_asset()
                .await
                .context("failed to read native asset from state")?;

            let result_denom = if asset_balance.asset == native_asset.to_ibc_prefixed() {
                native_asset.into()
            } else {
                ibc_to_trace(state, asset_balance.asset)
                    .await
                    .context("failed to map ibc prefixed asset to trace prefixed")?
                    .into()
            };
            Ok(AssetBalance {
                denom: result_denom,
                balance: asset_balance.balance,
            })
        })
        .try_buffered(16);
    stream.try_collect::<Vec<_>>().await
}

pub(crate) async fn balance_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::account::v1alpha1::BalanceResponse;
    let (address, snapshot, height) = match preprocess_request(&storage, &request, &params).await {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };

    let balances = match get_trace_prefixed_account_balances(&snapshot, address).await {
        Ok(balance) => balance,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed getting balance for provided address: {err:#}"),
                height,
                ..response::Query::default()
            };
        }
    };
    let payload = BalanceResponse {
        height: height.value(),
        balances,
    }
    .into_raw()
    .encode_to_vec()
    .into();
    response::Query {
        code: 0.into(),
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

pub(crate) async fn nonce_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::account::v1alpha1::NonceResponse;
    let (address, snapshot, height) = match preprocess_request(&storage, &request, &params).await {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };
    let nonce = match snapshot.get_account_nonce(address).await {
        Ok(nonce) => nonce,
        Err(err) => {
            return response::Query {
                code: 2.into(),
                info: "failed getting nonce for provided address".into(),
                log: format!("{err:?}"),
                height,
                ..response::Query::default()
            };
        }
    };
    let payload = NonceResponse {
        height: height.value(),
        nonce,
    }
    .into_raw()
    .encode_to_vec()
    .into();
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

async fn get_snapshot_and_height(storage: &Storage, height: Height) -> Result<(Snapshot, Height)> {
    let snapshot = match height.value() {
        0 => storage.latest_snapshot(),
        other => {
            let version = storage
                .latest_snapshot()
                .get_storage_version_by_height(other)
                .await
                .wrap_err("failed to get storage version from height")?;
            storage
                .snapshot(version)
                .ok_or_eyre("failed to get storage at version")?
        }
    };
    let height: Height = snapshot
        .get_block_height()
        .await
        .wrap_err("failed to get block height from snapshot")?
        .try_into()
        .wrap_err("internal u64 block height does not fit into tendermint i64 `Height`")?;
    Ok((snapshot, height))
}

async fn preprocess_request(
    storage: &Storage,
    request: &request::Query,
    params: &[(String, String)],
) -> Result<(Address, Snapshot, Height), response::Query> {
    let Some(address) = params
        .iter()
        .find_map(|(k, v)| (k == "account").then_some(v))
    else {
        return Err(response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: "path did not contain path parameter".into(),
            ..response::Query::default()
        });
    };
    let address = address
        .parse()
        .wrap_err("failed to parse argument as address")
        .map_err(|err| response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: format!("address could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    let (snapshot, height) = match get_snapshot_and_height(storage, request.height).await {
        Ok(tup) => tup,
        Err(err) => {
            return Err(response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed to query internal storage for snapshot and height: {err:#}"),
                ..response::Query::default()
            });
        }
    };
    Ok((address, snapshot, height))
}
