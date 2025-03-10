use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use astria_core::protocol::abci::AbciErrorCode;
use astria_eyre::eyre::WrapErr as _;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt as _,
    TryFutureExt as _,
};
use penumbra_tower_trace::v038::RequestExt as _;
use tendermint::v0_38::abci::{
    request,
    response::{
        self,
        Echo,
    },
    InfoRequest,
    InfoResponse,
};
use tokio::try_join;
use tower::Service;
use tower_abci::BoxError;
use tracing::{
    instrument,
    Instrument as _,
};

mod abci_query_router;

use astria_eyre::{
    anyhow_to_eyre,
    eyre::Result,
};

use crate::app::StateReadExt as _;

#[derive(Clone)]
pub(crate) struct Info {
    storage: Storage,
    query_router: abci_query_router::Router,
}

const ACCOUNT_BALANCE: &str = "accounts/balance/:account";
const ACCOUNT_NONCE: &str = "accounts/nonce/:account";
const ASSET_DENOM: &str = "asset/denom/:id";
const FEE_ALLOWED_ASSETS: &str = "asset/allowed_fee_assets";

const BRIDGE_ACCOUNT_LAST_TX_ID: &str = "bridge/account_last_tx_hash/:address";
const BRIDGE_ACCOUNT_INFO: &str = "bridge/account_info/:address";

const TRANSACTION_FEE: &str = "transaction/fee";

const FEES_COMPONENTS: &str = "fees/components";

impl Info {
    pub(crate) fn new(storage: Storage) -> Result<Self> {
        let mut query_router = abci_query_router::Router::new();

        // NOTE: Skipping error context because `InsertError` contains all required information.
        query_router.insert(ACCOUNT_BALANCE, crate::accounts::query::balance_request)?;
        query_router.insert(ACCOUNT_NONCE, crate::accounts::query::nonce_request)?;
        query_router.insert(ASSET_DENOM, crate::assets::query::denom_request)?;
        query_router.insert(
            FEE_ALLOWED_ASSETS,
            crate::fees::query::allowed_fee_assets_request,
        )?;
        query_router.insert(
            BRIDGE_ACCOUNT_LAST_TX_ID,
            crate::bridge::query::bridge_account_last_tx_hash_request,
        )?;
        query_router.insert(
            BRIDGE_ACCOUNT_INFO,
            crate::bridge::query::bridge_account_info_request,
        )?;
        query_router.insert(TRANSACTION_FEE, crate::fees::query::transaction_fee_request)?;
        query_router.insert(FEES_COMPONENTS, crate::fees::query::components)?;
        Ok(Self {
            storage,
            query_router,
        })
    }

    #[instrument(skip_all)]
    async fn handle_info_request(self, request: InfoRequest) -> Result<InfoResponse, BoxError> {
        match request {
            InfoRequest::Info(_) => {
                let snapshot = self.storage.latest_snapshot();

                let block_height_fut = snapshot.get_block_height().unwrap_or_else(|_| 0).map(Ok);
                let app_hash_fut = snapshot.root_hash().map_err(anyhow_to_eyre);
                let app_version_fut = snapshot.get_consensus_params();

                let (block_height, app_hash, maybe_consensus_params) =
                    try_join!(block_height_fut, app_hash_fut, app_version_fut)
                        .wrap_err("failed to fetch info")?;

                let app_version = maybe_consensus_params
                    .and_then(|consensus_params| consensus_params.version)
                    .map_or(1, |version_params| version_params.app);

                let response = InfoResponse::Info(response::Info {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    app_version,
                    last_block_height: u32::try_from(block_height)
                        .expect("block height must fit into u32")
                        .into(),
                    last_block_app_hash: app_hash.0.to_vec().try_into()?,
                    data: "astria_sequencer".to_string(),
                });
                Ok(response)
            }
            InfoRequest::Echo(echo) => Ok(InfoResponse::Echo(Echo {
                message: echo.message,
            })),
            InfoRequest::Query(req) => Ok(InfoResponse::Query(self.handle_abci_query(req).await)),
        }
    }

    /// Handles `abci_query` RPCs.
    async fn handle_abci_query(self, request: request::Query) -> response::Query {
        let (handler, params) = match self.query_router.at(&request.path) {
            Err(err) => {
                return response::Query {
                    code: tendermint::abci::Code::Err(AbciErrorCode::UNKNOWN_PATH.value()),
                    info: AbciErrorCode::UNKNOWN_PATH.info(),
                    log: format!("provided path `{}` is unknown: {err:#}", request.path),
                    ..response::Query::default()
                };
            }

            Ok(matchit::Match {
                value,
                params,
            }) => {
                let params = params
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                let handler = value.clone();
                (handler, params)
            }
        };
        handler.call(self.storage.clone(), request, params).await
    }
}

impl Service<InfoRequest> for Info {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    type Response = InfoResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: InfoRequest) -> Self::Future {
        let span = req.create_span();

        self.clone()
            .handle_info_request(req)
            .instrument(span)
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            account::v1::BalanceResponse,
            asset::v1::DenomResponse,
            fees::v1::FeeComponents,
            transaction::v1::action::{
                BridgeLock,
                BridgeSudoChange,
                BridgeTransfer,
                BridgeUnlock,
                FeeAssetChange,
                FeeChange,
                IbcRelayerChange,
                IbcSudoChange,
                Ics20Withdrawal,
                InitBridgeAccount,
                RecoverIbcClient,
                RollupDataSubmission,
                SudoAddressChange,
                Transfer,
                ValidatorUpdate,
            },
        },
    };
    use cnidarium::{
        StateDelta,
        StateWrite,
    };
    use penumbra_ibc::IbcRelay;
    use prost::Message as _;
    use tendermint::v0_38::abci::{
        request,
        InfoRequest,
        InfoResponse,
    };

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        address::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        app::StateWriteExt as _,
        assets::StateWriteExt as _,
        benchmark_and_test_utils::nria,
        fees::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    };

    #[tokio::test]
    async fn handle_balance_query() {
        use astria_core::{
            generated::astria::protocol::accounts::v1 as raw,
            protocol::account::v1::AssetBalance,
        };

        let storage = cnidarium::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let height = 99;
        let version = storage.latest_version().wrapping_add(1);
        let mut state = StateDelta::new(storage.latest_snapshot());
        state
            .put_storage_version_by_height(height, version)
            .unwrap();

        state.put_base_prefix("astria".to_string()).unwrap();
        state.put_native_asset(nria()).unwrap();
        state.put_ibc_asset(nria()).unwrap();

        let address = state
            .try_base_prefixed(&hex::decode("a034c743bed8f26cb8ee7b8db2230fd8347ae131").unwrap())
            .await
            .unwrap();

        let balance = 1000;
        state
            .put_account_balance(&address, &nria(), balance)
            .unwrap();
        state.put_block_height(height).unwrap();
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: format!("accounts/balance/{address}"),
            data: vec![].into(),
            height: u32::try_from(height).unwrap().into(),
            prove: false,
        });

        let response = {
            let storage = (*storage).clone();
            let info_service = Info::new(storage).unwrap();
            info_service
                .handle_info_request(info_request)
                .await
                .unwrap()
        };
        let query_response = match response {
            InfoResponse::Query(query) => query,
            other => panic!("expected InfoResponse::Query, got {other:?}"),
        };
        assert!(query_response.code.is_ok());

        let expected_balance = AssetBalance {
            denom: nria().into(),
            balance,
        };

        let balance_resp = BalanceResponse::try_from_raw(
            &raw::BalanceResponse::decode(query_response.value).unwrap(),
        )
        .unwrap();
        assert_eq!(balance_resp.balances.len(), 1);
        assert_eq!(balance_resp.balances[0], expected_balance);
        assert_eq!(balance_resp.height, height);
    }

    #[tokio::test]
    async fn handle_denom_query() {
        use astria_core::generated::astria::protocol::asset::v1 as raw;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());

        let denom: asset::TracePrefixed = "some/ibc/asset".parse().unwrap();
        let height = 99;
        state.put_block_height(height).unwrap();
        state.put_ibc_asset(denom.clone()).unwrap();
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: format!(
                "asset/denom/{}",
                hex::encode(denom.to_ibc_prefixed().as_bytes())
            ),
            data: vec![].into(),
            height: u32::try_from(height).unwrap().into(),
            prove: false,
        });

        let response = {
            let storage = (*storage).clone();
            let info_service = Info::new(storage).unwrap();
            info_service
                .handle_info_request(info_request)
                .await
                .unwrap()
        };
        let query_response = match response {
            InfoResponse::Query(query) => query,
            other => panic!("expected InfoResponse::Query, got {other:?}"),
        };
        assert!(query_response.code.is_ok());

        let denom_resp =
            DenomResponse::try_from_raw(&raw::DenomResponse::decode(query_response.value).unwrap())
                .unwrap();
        assert_eq!(denom_resp.height, height);
        assert_eq!(denom_resp.denom, denom.into());
    }

    #[tokio::test]
    async fn handle_allowed_fee_assets_query() {
        use astria_core::generated::astria::protocol::asset::v1 as raw;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());

        let assets = vec![
            "asset_0".parse::<asset::Denom>().unwrap(),
            "asset_1".parse::<asset::Denom>().unwrap(),
            "asset_2".parse::<asset::Denom>().unwrap(),
        ];
        let height = 99;

        for asset in &assets {
            state.put_allowed_fee_asset(asset).unwrap();
            assert!(
                state
                    .is_allowed_fee_asset(asset)
                    .await
                    .expect("checking for allowed fee asset should not fail"),
                "fee asset was expected to be allowed"
            );
        }
        state.put_block_height(height).unwrap();
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: "asset/allowed_fee_assets".to_string(),
            data: vec![].into(),
            height: u32::try_from(height).unwrap().into(),
            prove: false,
        });

        let response = {
            let storage = (*storage).clone();
            let info_service = Info::new(storage).unwrap();
            info_service
                .handle_info_request(info_request)
                .await
                .unwrap()
        };
        let query_response = match response {
            InfoResponse::Query(query) => query,
            other => panic!("expected InfoResponse::Query, got {other:?}"),
        };
        assert!(query_response.code.is_ok(), "{query_response:?}");

        let allowed_fee_assets_resp = raw::AllowedFeeAssetsResponse::decode(query_response.value)
            .unwrap()
            .try_to_native()
            .unwrap();
        assert_eq!(allowed_fee_assets_resp.height, height);
        assert_eq!(allowed_fee_assets_resp.fee_assets.len(), assets.len());
        for asset in &assets {
            assert!(
                allowed_fee_assets_resp
                    .fee_assets
                    .contains(&asset.to_ibc_prefixed().into()),
                "expected asset_id to be in allowed fee assets"
            );
        }
    }

    #[tokio::test]
    async fn handle_fee_components() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());

        let height = 99;

        state.put_block_height(height).unwrap();
        write_all_the_fees(&mut state);
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: "fees/components".to_string(),
            data: vec![].into(),
            height: u32::try_from(height).unwrap().into(),
            prove: false,
        });

        let response = {
            let storage = (*storage).clone();
            let info_service = Info::new(storage).unwrap();
            info_service
                .handle_info_request(info_request)
                .await
                .unwrap()
        };
        let query_response = match response {
            InfoResponse::Query(query) => query,
            other => panic!("expected InfoResponse::Query, got {other:?}"),
        };
        assert!(query_response.code.is_ok(), "{query_response:?}");

        let actual_fees =
            serde_json::from_slice::<serde_json::Value>(&query_response.value).unwrap();

        assert_json_diff::assert_json_eq!(expected_fees(), actual_fees);
    }

    fn expected_fees() -> serde_json::Value {
        serde_json::json!({
              "bridge_lock": {
                "base": 1,
                "multiplier": 1
              },
              "bridge_sudo_change": {
                "base": 3,
                "multiplier": 3
              },
              "bridge_unlock": {
                "base": 2,
                "multiplier": 2
              },
              "bridge_transfer": {
                "base": 13,
                "multiplier": 13
              },
              "fee_asset_change": {
                "base": 4,
                "multiplier": 4
              },
              "fee_change": {
                "base": 5,
                "multiplier": 5
              },
              "ibc_relay": {
                "base": 7,
                "multiplier": 7
              },
              "ibc_relayer_change": {
                "base": 8,
                "multiplier": 8
              },
              "ibc_sudo_change": {
                "base": 9,
                "multiplier": 9
              },
              "ics20_withdrawal": {
                "base": 10,
                "multiplier": 10
              },
              "init_bridge_account": {
                "base": 6,
                "multiplier": 6
              },
              "recover_ibc_client": {
                "base": 0,
                "multiplier": 0
              },
              "rollup_data_submission": {
                "base": 11,
                "multiplier": 11
              },
              "sudo_address_change": {
                "base": 12,
                "multiplier": 12
              },
              "transfer": {
                "base": 13,
                "multiplier": 13
              },
              "validator_update": {
                "base": 14,
                "multiplier": 14
            }
        })
    }

    fn write_all_the_fees<S: StateWrite>(mut state: S) {
        state
            .put_fees(FeeComponents::<BridgeLock>::new(1, 1))
            .unwrap();
        state
            .put_fees(FeeComponents::<BridgeUnlock>::new(2, 2))
            .unwrap();
        state
            .put_fees(FeeComponents::<BridgeTransfer>::new(13, 13))
            .unwrap();
        state
            .put_fees(FeeComponents::<BridgeSudoChange>::new(3, 3))
            .unwrap();
        state
            .put_fees(FeeComponents::<FeeAssetChange>::new(4, 4))
            .unwrap();
        state
            .put_fees(FeeComponents::<FeeChange>::new(5, 5))
            .unwrap();
        state
            .put_fees(FeeComponents::<InitBridgeAccount>::new(6, 6))
            .unwrap();
        state
            .put_fees(FeeComponents::<IbcRelay>::new(7, 7))
            .unwrap();
        state
            .put_fees(FeeComponents::<IbcRelayerChange>::new(8, 8))
            .unwrap();
        state
            .put_fees(FeeComponents::<IbcSudoChange>::new(9, 9))
            .unwrap();
        state
            .put_fees(FeeComponents::<Ics20Withdrawal>::new(10, 10))
            .unwrap();
        state
            .put_fees(FeeComponents::<RecoverIbcClient>::new(0, 0))
            .unwrap();
        state
            .put_fees(FeeComponents::<RollupDataSubmission>::new(11, 11))
            .unwrap();
        state
            .put_fees(FeeComponents::<SudoAddressChange>::new(12, 12))
            .unwrap();
        state
            .put_fees(FeeComponents::<Transfer>::new(13, 13))
            .unwrap();
        state
            .put_fees(FeeComponents::<ValidatorUpdate>::new(14, 14))
            .unwrap();
    }
}
