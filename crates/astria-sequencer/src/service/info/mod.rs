use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::Context as _;
use astria_core::protocol::abci::AbciErrorCode;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
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
use tower::Service;
use tower_abci::BoxError;
use tracing::{
    instrument,
    Instrument as _,
};

mod abci_query_router;

use crate::state_ext::StateReadExt;

#[derive(Clone)]
pub(crate) struct Info {
    storage: Storage,
    query_router: abci_query_router::Router,
}

impl Info {
    pub(crate) fn new(storage: Storage) -> anyhow::Result<Self> {
        let mut query_router = abci_query_router::Router::new();
        query_router
            .insert(
                "accounts/balance/:account",
                crate::accounts::query::balance_request,
            )
            .context("invalid path: `accounts/balance/:account`")?;
        query_router
            .insert(
                "accounts/nonce/:account",
                crate::accounts::query::nonce_request,
            )
            .context("invalid path: `accounts/nonce/:account`")?;
        query_router
            .insert("asset/denom/:id", crate::asset::query::denom_request)
            .context("invalid path: `asset/denom/:id`")?;
        query_router
            .insert(
                "asset/allowed_fee_asset_ids",
                crate::asset::query::allowed_fee_asset_ids_request,
            )
            .context("invalid path: `asset/allowed_fee_asset_ids`")?;
        query_router
            .insert(
                "bridge/account_last_tx_hash/:address",
                crate::bridge::query::bridge_account_last_tx_hash_request,
            )
            .context("invalid path: `bridge/account_last_tx_hash/:address`")?;
        Ok(Self {
            storage,
            query_router,
        })
    }

    #[instrument(skip(self))]
    async fn handle_info_request(self, request: InfoRequest) -> Result<InfoResponse, BoxError> {
        match request {
            InfoRequest::Info(_) => {
                let block_height = self
                    .storage
                    .latest_snapshot()
                    .get_block_height()
                    .await
                    .unwrap_or(0);
                let app_hash = self
                    .storage
                    .latest_snapshot()
                    .root_hash()
                    .await
                    .context("failed to get app hash")?;

                let response = InfoResponse::Info(response::Info {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    app_version: 1,
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
                    code: AbciErrorCode::UNKNOWN_PATH.into(),
                    info: AbciErrorCode::UNKNOWN_PATH.to_string(),
                    log: format!("provided path `{}` is unknown: {err:?}", request.path),
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
mod test {
    use astria_core::{
        primitive::v1::asset::{
            self,
            denom::TracePrefixed,
            DEFAULT_NATIVE_ASSET_DENOM,
        },
        protocol::{
            account::v1alpha1::BalanceResponse,
            asset::v1alpha1::DenomResponse,
        },
    };
    use cnidarium::StateDelta;
    use prost::Message as _;
    use tendermint::v0_38::abci::{
        request,
        InfoRequest,
        InfoResponse,
    };

    use super::Info;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        asset::{
            get_native_asset,
            initialize_native_asset,
            state_ext::StateWriteExt,
        },
        state_ext::{
            StateReadExt,
            StateWriteExt as _,
        },
    };

    #[tokio::test]
    async fn handle_balance_query() {
        use astria_core::{
            generated::protocol::account::v1alpha1 as raw,
            protocol::account::v1alpha1::AssetBalance,
        };

        let storage = cnidarium::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let height = 99;
        let version = storage.latest_version().wrapping_add(1);
        let mut state = StateDelta::new(storage.latest_snapshot());
        state.put_storage_version_by_height(height, version);

        initialize_native_asset(DEFAULT_NATIVE_ASSET_DENOM);

        let address = crate::address::try_base_prefixed(
            &hex::decode("a034c743bed8f26cb8ee7b8db2230fd8347ae131").unwrap(),
        )
        .unwrap();

        let balance = 1000;
        state
            .put_account_balance(address, get_native_asset().id(), balance)
            .unwrap();
        state.put_block_height(height);
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
            denom: get_native_asset().clone(),
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
        use astria_core::generated::protocol::asset::v1alpha1 as raw;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());

        let denom = "some/ibc/asset".parse::<TracePrefixed>().unwrap();
        let id = denom.id();
        let height = 99;
        state.put_block_height(height);
        state.put_ibc_asset(id, &denom).unwrap();
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: format!("asset/denom/{}", hex::encode(id)),
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
    async fn handle_allowed_fee_asset_ids_query() {
        use astria_core::generated::protocol::asset::v1alpha1 as raw;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());

        let asset_ids = vec![
            asset::Id::from_str_unchecked("asset_0"),
            asset::Id::from_str_unchecked("asset_1"),
            asset::Id::from_str_unchecked("asset_2"),
        ];
        let height = 99;

        for &asset_id in &asset_ids {
            state.put_allowed_fee_asset(asset_id);
            assert!(
                state
                    .is_allowed_fee_asset(asset_id)
                    .await
                    .expect("checking for allowed fee asset should not fail"),
                "fee asset was expected to be allowed"
            );
        }
        state.put_block_height(height);
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: "asset/allowed_fee_asset_ids".to_string(),
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

        let allowed_fee_assets_resp = raw::AllowedFeeAssetIdsResponse::decode(query_response.value)
            .unwrap()
            .try_to_native()
            .unwrap();
        assert_eq!(allowed_fee_assets_resp.height, height);
        assert_eq!(allowed_fee_assets_resp.fee_asset_ids.len(), asset_ids.len());
        for asset_id in asset_ids {
            assert!(
                allowed_fee_assets_resp.fee_asset_ids.contains(&asset_id),
                "expected asset_id to be in allowed fee assets"
            );
        }
    }
}
