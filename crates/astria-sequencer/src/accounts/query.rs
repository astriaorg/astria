use anyhow::Context as _;
use penumbra_storage::{
    Snapshot,
    Storage,
};
use tendermint::{
    abci::{
        request,
        response,
    },
    block::Height,
};

use crate::{
    accounts::{
        state_ext::StateReadExt as _,
        types::Address,
    },
    service::info::AbciCode,
    state_ext::StateReadExt as _,
};

pub(crate) async fn balance_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_proto::{
        native::sequencer::BalanceResponse,
        // sequencer::v1alpha1::BalanceResponse,
        Message as _,
    };
    let (address, snapshot, height) = match preprocess_request(&storage, &request, &params).await {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };
    let balance = match snapshot.get_account_balance(&address).await {
        Ok(balance) => balance,
        Err(err) => {
            return response::Query {
                code: AbciCode::INVALID_PARAMETER.into(),
                info: format!("{}", AbciCode::INVALID_PARAMETER),
                log: format!("failed getting balance for provided address: {err:?}"),
                height,
                ..response::Query::default()
            };
        }
    };
    let payload = BalanceResponse {
        account: address.0,
        height: height.value(),
        balance: balance.0,
    }
    .into_proto()
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
    use astria_proto::{
        native::sequencer::NonceResponse,
        Message as _,
    };
    let (address, snapshot, height) = match preprocess_request(&storage, &request, &params).await {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };
    let nonce = match snapshot.get_account_nonce(&address).await {
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
        account: address.0,
        height: height.value(),
        nonce: nonce.0,
    }
    .into_proto()
    .encode_to_vec()
    .into();
    response::Query {
        code: AbciCode::OK.into(),
        info: format!("{}", AbciCode::OK),
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

async fn get_snapshot_and_height(
    storage: &Storage,
    height: Height,
) -> anyhow::Result<(Snapshot, Height)> {
    let snapshot = match height.value() {
        0 => storage.latest_snapshot(),
        other => {
            let version = storage
                .latest_snapshot()
                .get_storage_version_by_height(other)
                .await
                .context("failed to get storage version from height")?;
            storage
                .snapshot(version)
                .context("failed to get storage at version")?
        }
    };
    let height: Height = snapshot
        .get_block_height()
        .await
        .context("failed to get block height from snapshot")?
        .try_into()
        .context("internal u64 block height does not fit into tendermint i64 `Height`")?;
    Ok((snapshot, height))
}

async fn preprocess_request(
    storage: &Storage,
    request: &request::Query,
    params: &[(String, String)],
) -> anyhow::Result<(Address, Snapshot, Height), response::Query> {
    let Some(address) = params
        .iter()
        .find_map(|(k, v)| (k == "account").then_some(v))
    else {
        return Err(response::Query {
            code: AbciCode::INVALID_PARAMETER.into(),
            info: format!("{}", AbciCode::INVALID_PARAMETER),
            log: "path did not contain path parameter".into(),
            ..response::Query::default()
        });
    };
    let address = match Address::try_from_str(address) {
        Ok(address) => address,
        Err(err) => {
            return Err(response::Query {
                code: AbciCode::INVALID_PARAMETER.into(),
                info: format!("{}", AbciCode::INVALID_PARAMETER),
                log: format!(
                    "account public key could not be constructed from provided paratemer: {err:?}"
                ),
                ..response::Query::default()
            });
        }
    };
    let (snapshot, height) = match get_snapshot_and_height(storage, request.height).await {
        Ok(tup) => tup,
        Err(err) => {
            return Err(response::Query {
                code: AbciCode::INTERNAL_ERROR.into(),
                info: format!("{}", AbciCode::INTERNAL_ERROR),
                log: format!("failed to query internal storage for snapshot and height: {err:?}"),
                ..response::Query::default()
            });
        }
    };
    Ok((address, snapshot, height))
}
