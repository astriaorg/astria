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

impl Info {
    pub(crate) fn new(storage: Storage) -> Result<Self> {
        let mut query_router = abci_query_router::Router::new();
        query_router
            .insert(
                "accounts/balance/:account",
                crate::accounts::query::balance_request,
            )
            .wrap_err("invalid path: `accounts/balance/:account`")?;
        query_router
            .insert(
                "accounts/nonce/:account",
                crate::accounts::query::nonce_request,
            )
            .wrap_err("invalid path: `accounts/nonce/:account`")?;
        query_router
            .insert("asset/denom/:id", crate::assets::query::denom_request)
            .wrap_err("invalid path: `asset/denom/:id`")?;
        query_router
            .insert(
                "asset/allowed_fee_assets",
                crate::fees::query::allowed_fee_assets_request,
            )
            .wrap_err("invalid path: `asset/allowed_fee_asset_ids`")?;
        query_router
            .insert(
                "bridge/account_last_tx_hash/:address",
                crate::bridge::query::bridge_account_last_tx_hash_request,
            )
            .wrap_err("invalid path: `bridge/account_last_tx_hash/:address`")?;
        query_router
            .insert(
                "transaction/fee",
                crate::fees::query::transaction_fee_request,
            )
            .wrap_err("invalid path: `transaction/fee`")?;
        query_router
            .insert(
                "bridge/account_info/:address",
                crate::bridge::query::bridge_account_info_request,
            )
            .wrap_err("invalid path: `bridge/account_info/:address`")?;
        query_router
            .insert(
                "authority/validator_name/:address",
                crate::authority::query::validator_name_request,
            )
            .wrap_err("invalid path: `authority/validator_name/:address`")?;
        Ok(Self {
            storage,
            query_router,
        })
    }

    #[instrument(skip_all)]
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
                    .map_err(anyhow_to_eyre)
                    .wrap_err("failed to get app hash")?;

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
    use std::collections::BTreeMap;

    use astria_core::{
        primitive::v1::asset,
        protocol::{
            account::v1::BalanceResponse,
            asset::v1::DenomResponse,
            transaction::v1::action::{
                ValidatorUpdate,
                ValidatorUpdateV2,
            },
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
        accounts::StateWriteExt as _,
        address::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        app::StateWriteExt as _,
        assets::StateWriteExt as _,
        authority::{
            StateWriteExt as _,
            ValidatorNames,
            ValidatorSet,
        },
        fees::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        test_utils::{
            astria_address,
            verification_key,
        },
    };

    #[tokio::test]
    async fn handle_balance_query() {
        use astria_core::{
            generated::protocol::accounts::v1 as raw,
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
        state.put_native_asset(crate::test_utils::nria()).unwrap();
        state.put_ibc_asset(crate::test_utils::nria()).unwrap();

        let address = state
            .try_base_prefixed(&hex::decode("a034c743bed8f26cb8ee7b8db2230fd8347ae131").unwrap())
            .await
            .unwrap();

        let balance = 1000;
        state
            .put_account_balance(&address, &crate::test_utils::nria(), balance)
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
            denom: crate::test_utils::nria().into(),
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
        use astria_core::generated::protocol::asset::v1 as raw;

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
        use astria_core::generated::protocol::asset::v1 as raw;

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
        assert!(query_response.code.is_ok());

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
    async fn handle_validator_name_query() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let mut state = StateDelta::new(storage.latest_snapshot());
        let verification_key = verification_key(1);
        let height = 0u32;
        let power = 100;

        let inner_validator_update = ValidatorUpdate {
            power,
            verification_key: verification_key.clone(),
        };

        let validator_update = ValidatorUpdateV2 {
            verification_key: verification_key.clone(),
            power,
            name: "validator_name".to_string(),
        };

        let mut validator_names = ValidatorNames::new(BTreeMap::new());
        validator_names.insert(
            &validator_update.verification_key,
            validator_update.name.clone(),
        );
        state.put_validator_names(validator_names).unwrap();
        let mut validator_set = ValidatorSet::new(BTreeMap::new());
        validator_set.insert(inner_validator_update);
        state.put_validator_set(validator_set).unwrap();
        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: format!(
                "authority/validator_name/{}",
                astria_address(verification_key.address_bytes())
            ),
            data: vec![].into(),
            height: height.into(),
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

        let validator_name_resp = String::from_utf8(query_response.value.to_vec()).unwrap();
        assert_eq!(validator_name_resp, validator_update.name);
    }
}
