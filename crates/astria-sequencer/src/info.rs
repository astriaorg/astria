use std::{
    collections::VecDeque,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::Result;
use futures::{
    Future,
    FutureExt,
};
use penumbra_storage::Storage;
use tendermint::{
    abci::{
        request,
        response::{
            self,
            Echo,
            Info,
            SetOption,
        },
        InfoRequest,
        InfoResponse,
    },
    block::Height,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

use crate::{
    accounts::query::QueryHandler,
    state_ext::StateReadExt,
};

#[derive(Clone)]
pub struct InfoService {
    storage: Storage,
}

impl InfoService {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
        }
    }
}

impl Service<InfoRequest> for InfoService {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    type Response = InfoResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: InfoRequest) -> Self::Future {
        info!("got info request: {:?}", req);
        handle_info_request(self.storage.clone(), req).boxed()
    }
}

async fn handle_info_request(
    storage: Storage,
    request: InfoRequest,
) -> Result<InfoResponse, BoxError> {
    match &request {
        InfoRequest::Info(_) => {
            let response = InfoResponse::Info(Info {
                version: "0.1.0".to_string(),
                app_version: 1,
                last_block_height: Default::default(),
                last_block_app_hash: Default::default(),
                data: "astria_sequencer".to_string(),
            });
            Ok(response)
        }
        InfoRequest::Echo(echo) => Ok(InfoResponse::Echo(Echo {
            message: echo.message.clone(),
        })),
        InfoRequest::Query(req) => Ok(InfoResponse::Query(handle_query(storage, req).await?)),
        // this was removed after v0.34
        InfoRequest::SetOption(_) => Ok(InfoResponse::SetOption(SetOption {
            code: Default::default(),
            log: "SetOption is not supported".to_string(),
            info: "SetOption is not supported".to_string(),
        })),
    }
}

/// handles queries in the form of [`component/arg1/arg2/...`]
/// for example, to query an account balance: [`accounts/balance/0x1234...`]
async fn handle_query(storage: Storage, request: &request::Query) -> Result<response::Query> {
    // note: request::Query also has a `data` field, which we ignore here
    let query = decode_query(&request.path)?;

    // TODO: handle height requests
    let key = request.path.clone().into_bytes();
    let value = match query {
        Query::AccountsQuery(request) => {
            let handler = QueryHandler::new();
            handler.handle(storage.latest_snapshot(), request).await?
        }
    }
    .to_bytes()?;

    let height = storage.latest_snapshot().get_block_height().await?;

    Ok(response::Query {
        key: key.into(),
        value: value.into(),
        height: Height::from(height as u32),
        ..Default::default()
    })
}

pub enum Query {
    AccountsQuery(crate::accounts::query::QueryRequest),
}

fn decode_query(path: &str) -> Result<Query> {
    let mut parts: VecDeque<&str> = path.split('/').collect();

    let Some(component) = parts.pop_front() else {
        return Err(anyhow::anyhow!("invalid query path; missing component: {}", path));
    };

    match component {
        "accounts" => {
            let request = crate::accounts::query::QueryRequest::decode(parts)?;
            Ok(Query::AccountsQuery(request))
        }
        _ => Err(anyhow::anyhow!("invalid query path: {}", path)),
    }
}
