use std::{
    fmt::Display,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::Context as _;
use futures::{
    Future,
    FutureExt,
};
use penumbra_storage::Storage;
use penumbra_tower_trace::RequestExt as _;
use tendermint::{
    abci::{
        request,
        response::{
            self,
            Echo,
            SetOption,
        },
        Code,
        InfoRequest,
        InfoResponse,
    },
    block::Height,
    AppHash,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::{
    instrument,
    Instrument,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AbciCode(u32);
impl AbciCode {
    pub(crate) const INTERNAL_ERROR: Self = Self(3);
    pub(crate) const INVALID_PARAMETER: Self = Self(2);
    pub(crate) const OK: Self = Self(0);
    pub(crate) const UNKNOWN_PATH: Self = Self(1);

    pub(crate) fn info(self) -> Option<&'static str> {
        match self.0 {
            0 => Some("Ok"),
            1 => Some("provided path is unknown"),
            2 => Some("one or more path parameters were invalid"),
            3 => Some("an internal server error occured"),
            _ => None,
        }
    }
}
impl Display for AbciCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info().unwrap_or("<unknown abci code>"))
    }
}

impl From<AbciCode> for Code {
    fn from(value: AbciCode) -> Self {
        value.0.into()
    }
}

#[derive(Clone)]
pub(crate) struct Info {
    storage: Storage,
    query_router: matchit::Router<BoxedAbciQueryHandler>,
}

impl Info {
    pub(crate) fn new(storage: Storage) -> anyhow::Result<Self> {
        let mut query_router = matchit::Router::new();
        query_router
            .insert(
                "accounts/balance/:account",
                BoxedAbciQueryHandler::from_handler(crate::accounts::query::balance_request),
            )
            .context("invalid path: `accounts/balance/:account`")?;
        query_router
            .insert(
                "accounts/nonce/:account",
                BoxedAbciQueryHandler::from_handler(crate::accounts::query::nonce_request),
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
                let response = InfoResponse::Info(response::Info {
                    version: "0.1.0".to_string(),
                    app_version: 1,
                    last_block_height: Height::default(),
                    last_block_app_hash: AppHash::default(),
                    data: "astria_sequencer".to_string(),
                });
                Ok(response)
            }
            InfoRequest::Echo(echo) => Ok(InfoResponse::Echo(Echo {
                message: echo.message,
            })),
            InfoRequest::Query(req) => Ok(InfoResponse::Query(self.handle_abci_query(req).await)),
            // this was removed after v0.34
            InfoRequest::SetOption(_) => Ok(InfoResponse::SetOption(SetOption {
                code: Code::default(),
                log: "SetOption is not supported".to_string(),
                info: "SetOption is not supported".to_string(),
            })),
        }
    }

    /// Handles `abci_query` RPCs.
    async fn handle_abci_query(self, request: request::Query) -> response::Query {
        let (handler, params) = match self.query_router.at(&request.path) {
            Err(err) => {
                return response::Query {
                    code: AbciCode::UNKNOWN_PATH.into(),
                    info: format!("{}", AbciCode::UNKNOWN_PATH),
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

struct BoxedAbciQueryHandler(Box<dyn ErasedAbciQueryHandler>);

impl BoxedAbciQueryHandler {
    fn from_handler<H>(handler: H) -> Self
    where
        H: AbciQueryHandler,
    {
        Self(Box::new(MakeErasedAbciQueryHandler {
            handler,
        }))
    }

    async fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> response::Query {
        self.0.call(storage, request, params).await
    }
}

impl Clone for BoxedAbciQueryHandler {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

trait ErasedAbciQueryHandler: Send {
    fn clone_box(&self) -> Box<dyn ErasedAbciQueryHandler>;

    fn call(
        self: Box<Self>,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>>;
}

struct MakeErasedAbciQueryHandler<H> {
    handler: H,
}

impl<H> Clone for MakeErasedAbciQueryHandler<H>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
        }
    }
}

impl<H> ErasedAbciQueryHandler for MakeErasedAbciQueryHandler<H>
where
    H: AbciQueryHandler + Clone + Send + 'static,
{
    fn clone_box(&self) -> Box<dyn ErasedAbciQueryHandler> {
        Box::new(self.clone())
    }

    fn call(
        self: Box<Self>,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>> {
        self.handler.call(storage, request, params)
    }
}

trait AbciQueryHandler: Clone + Send + Sized + 'static {
    fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>>;
}

impl<F, Fut> AbciQueryHandler for F
where
    F: FnOnce(Storage, request::Query, Vec<(String, String)>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = response::Query> + Send,
{
    fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>> {
        Box::pin(async move { self(storage, request, params).await })
    }
}

#[cfg(test)]
mod test {
    use astria_proto::native::sequencer::Address;
    use penumbra_storage::StateDelta;
    use tendermint::abci::{
        request,
        InfoRequest,
        InfoResponse,
    };

    use super::Info;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn handle_query() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let height = 99;
        let version = storage.latest_version().wrapping_add(1);
        let mut state = StateDelta::new(storage.latest_snapshot());
        state.put_storage_version_by_height(height, version);

        let address = Address::try_from_slice(
            &*hex::decode("a034c743bed8f26cb8ee7b8db2230fd8347ae131").unwrap(),
        )
        .unwrap();
        state.put_account_balance(address, 1000.into()).unwrap();
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
