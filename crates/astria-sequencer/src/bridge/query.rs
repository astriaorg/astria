use anyhow::Context as _;
use astria_core::{
    primitive::v1::Address,
    protocol::{
        abci::AbciErrorCode,
        bridge::v1alpha1::BridgeAccountInfo,
    },
};
use cnidarium::Storage;
use prost::Message as _;
use tendermint::abci::{
    request,
    response,
    Code,
};

use crate::{
    assets::StateReadExt as _,
    bridge::StateReadExt as _,
    state_ext::StateReadExt as _,
};

fn error_query_response(
    err: Option<anyhow::Error>,
    code: AbciErrorCode,
    msg: &str,
) -> response::Query {
    let log = match err {
        Some(err) => format!("{msg}: {err:#}"),
        None => msg.into(),
    };
    response::Query {
        code: Code::Err(code.value()),
        info: code.info(),
        log,
        ..response::Query::default()
    }
}

async fn get_bridge_account_info(
    snapshot: cnidarium::Snapshot,
    address: Address,
) -> anyhow::Result<Option<BridgeAccountInfo>, response::Query> {
    let rollup_id = match snapshot.get_bridge_account_rollup_id(&address).await {
        Ok(Some(rollup_id)) => rollup_id,
        Ok(None) => {
            return Ok(None);
        }
        Err(err) => {
            return Err(error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get rollup id",
            ));
        }
    };

    let ibc_asset = match snapshot.get_bridge_account_ibc_asset(&address).await {
        Ok(asset) => asset,
        Err(err) => {
            return Err(error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get bridge asset",
            ));
        }
    };

    let trace_asset = match snapshot.map_ibc_to_trace_prefixed_asset(ibc_asset).await {
        Ok(Some(trace_asset)) => trace_asset,
        Ok(None) => {
            return Err(error_query_response(
                None,
                AbciErrorCode::INTERNAL_ERROR,
                "failed to map ibc asset to trace asset; asset does not exist in state",
            ));
        }
        Err(err) => {
            return Err(error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to map ibc asset to trace asset",
            ));
        }
    };

    let sudo_address = match snapshot.get_bridge_account_sudo_address(&address).await {
        Ok(Some(sudo_address)) => sudo_address,
        Ok(None) => {
            return Err(error_query_response(
                None,
                AbciErrorCode::INTERNAL_ERROR,
                "sudo address not set",
            ));
        }
        Err(err) => {
            return Err(error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get sudo address",
            ));
        }
    };

    let withdrawer_address = match snapshot
        .get_bridge_account_withdrawer_address(&address)
        .await
    {
        Ok(Some(withdrawer_address)) => withdrawer_address,
        Ok(None) => {
            return Err(error_query_response(
                None,
                AbciErrorCode::INTERNAL_ERROR,
                "withdrawer address not set",
            ));
        }
        Err(err) => {
            return Err(error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get withdrawer address",
            ));
        }
    };

    Ok(Some(BridgeAccountInfo {
        rollup_id,
        asset: trace_asset.into(),
        sudo_address,
        withdrawer_address,
    }))
}

pub(crate) async fn bridge_account_info_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::bridge::v1alpha1::BridgeAccountInfoResponse;

    let address = match preprocess_request(&params) {
        Ok(tup) => tup,
        Err(err_rsp) => return err_rsp,
    };

    let snapshot = storage.latest_snapshot();
    let height = match snapshot.get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get block height",
            );
        }
    };

    let info = match get_bridge_account_info(snapshot, address).await {
        Ok(info) => info,
        Err(err) => {
            return err;
        }
    };

    let resp = BridgeAccountInfoResponse {
        height,
        info,
    };

    let payload = resp.into_raw().encode_to_vec().into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: 0.into(),
        key: request.path.clone().into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

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
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get block height",
            );
        }
    };

    let resp = match snapshot
        .get_last_transaction_hash_for_bridge_account(&address)
        .await
    {
        Ok(Some(tx_hash)) => BridgeAccountLastTxHashResponse {
            height,
            tx_hash: Some(tx_hash),
        },
        Ok(None) => BridgeAccountLastTxHashResponse {
            height,
            tx_hash: None,
        },
        Err(err) => {
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed getting balance for provided address",
            );
        }
    };
    let payload = resp.into_raw().encode_to_vec().into();

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
        return Err(error_query_response(
            None,
            AbciErrorCode::INVALID_PARAMETER,
            "path did not contain address parameter",
        ));
    };
    let address = address
        .parse()
        .context("failed to parse argument as address")
        .map_err(|err| response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: format!("address could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    Ok(address)
}

#[cfg(test)]
mod test {
    use astria_core::{
        generated::protocol::bridge::v1alpha1::BridgeAccountInfoResponse as RawBridgeAccountInfoResponse,
        primitive::v1::RollupId,
        protocol::bridge::v1alpha1::BridgeAccountInfoResponse,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        assets::StateWriteExt as _,
        bridge::StateWriteExt as _,
        state_ext::StateWriteExt as _,
        test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
    };

    #[tokio::test]
    async fn bridge_account_info_request_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        let asset: astria_core::primitive::v1::asset::Denom = "test".parse().unwrap();
        let rollup_id = RollupId::from_unhashed_bytes("test");
        let bridge_address = astria_address(&[0u8; 20]);
        let sudo_address = astria_address(&[1u8; 20]);
        let withdrawer_address = astria_address(&[2u8; 20]);
        state.put_block_height(1);
        state.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state
            .put_ibc_asset(asset.as_trace_prefixed().unwrap())
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, &asset)
            .unwrap();
        state.put_bridge_account_sudo_address(&bridge_address, &sudo_address);
        state.put_bridge_account_withdrawer_address(&bridge_address, &withdrawer_address);
        storage.commit(state).await.unwrap();

        let query = request::Query {
            data: vec![].into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };

        let params = vec![("address".to_string(), bridge_address.to_string())];
        let resp = bridge_account_info_request(storage.clone(), query, params).await;
        assert_eq!(resp.code, 0.into(), "{}", resp.log);

        let proto = RawBridgeAccountInfoResponse::decode(resp.value).unwrap();
        let native = BridgeAccountInfoResponse::try_from_raw(proto).unwrap();
        let expected = BridgeAccountInfoResponse {
            height: 1,
            info: Some(BridgeAccountInfo {
                rollup_id,
                asset,
                sudo_address,
                withdrawer_address,
            }),
        };
        assert_eq!(native, expected);
    }
}
