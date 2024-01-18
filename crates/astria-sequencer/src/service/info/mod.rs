use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::Context as _;
use astria_core::sequencer::v1alpha1::AbciErrorCode;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use penumbra_tower_trace::v037::RequestExt as _;
use tendermint::v0_37::abci::{
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
    Instrument,
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
    use astria_core::sequencer::v1alpha1::{
        asset::{
            Denom,
            DEFAULT_NATIVE_ASSET_DENOM,
        },
        Address,
    };
    use cnidarium::StateDelta;
    use tendermint::v0_37::abci::{
        request,
        InfoRequest,
        InfoResponse,
    };

    use super::Info;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        asset::{
            get_native_asset,
            NATIVE_ASSET,
        },
        state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn handle_query() {
        let storage = cnidarium::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let height = 99;
        let version = storage.latest_version().wrapping_add(1);
        let mut state = StateDelta::new(storage.latest_snapshot());
        state.put_storage_version_by_height(height, version);

        let _ = NATIVE_ASSET.set(Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM));

        let address = Address::try_from_slice(
            &hex::decode("a034c743bed8f26cb8ee7b8db2230fd8347ae131").unwrap(),
        )
        .unwrap();
        state
            .put_account_balance(address, get_native_asset().id(), 1000)
            .unwrap();
        state.put_block_height(height);

        storage.commit(state).await.unwrap();

        let info_request = InfoRequest::Query(request::Query {
            path: "accounts/balance/a034c743bed8f26cb8ee7b8db2230fd8347ae131".to_string(),
            data: vec![].into(),
            height: u32::try_from(height).unwrap().into(),
            prove: false,
        });

        let query_response = match {
            let storage = (*storage).clone();
            let info_service = Info::new(storage).unwrap();
            info_service
                .handle_info_request(info_request)
                .await
                .unwrap()
        } {
            InfoResponse::Query(query) => query,
            other => panic!("expected InfoResponse::Query, got {other:?}"),
        };
        assert!(query_response.code.is_ok());
    }
}
