use std::{
    collections::VecDeque,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::{
    bail,
    Context as _,
};
use borsh::BorshSerialize as _;
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
    warn,
    Instrument,
};

use crate::{
    accounts::query::QueryHandler,
    state_ext::StateReadExt,
};

#[derive(Clone)]
pub(crate) struct Info {
    storage: Storage,
}

impl Info {
    pub(crate) fn new(storage: Storage) -> Self {
        Self {
            storage,
        }
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

        handle_info_request(self.storage.clone(), req)
            .instrument(span)
            .boxed()
    }
}

#[instrument(skip(storage))]
async fn handle_info_request(
    storage: Storage,
    request: InfoRequest,
) -> Result<InfoResponse, BoxError> {
    match &request {
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
            message: echo.message.clone(),
        })),
        InfoRequest::Query(req) => Ok(InfoResponse::Query(
            handle_query(storage, req)
                .await
                .context("failed handling query request")?,
        )),
        // this was removed after v0.34
        InfoRequest::SetOption(_) => Ok(InfoResponse::SetOption(SetOption {
            code: Code::default(),
            log: "SetOption is not supported".to_string(),
            info: "SetOption is not supported".to_string(),
        })),
    }
}

/// handles queries in the form of [`component/arg1/arg2/...`]
/// for example, to query an account balance: [`accounts/balance/0x1234...`]
async fn handle_query(
    storage: Storage,
    request: &request::Query,
) -> anyhow::Result<response::Query> {
    // note: request::Query also has a `data` field, which we ignore here
    let query = decode_query(&request.path).context("failed to decode query")?;

    let state = match request.height.value() {
        0 => storage.latest_snapshot(),
        height => storage
            .snapshot(height)
            .context("failed to get storage at height")?,
    };

    let key = request.path.clone().into_bytes();
    let value = match query {
        Query::AccountsQuery(request) => {
            let handler = QueryHandler::new();
            handler
                .handle(state, request)
                .await
                .context("failed to handle accounts query")?
        }
    }
    .try_to_vec()
    .context("failed serializing query response")?;

    let height = storage
        .latest_snapshot()
        .get_block_height()
        .await
        .context("failed to get block from latest snapshot")?;

    let height = match u32::try_from(height) {
        Ok(height) => height,
        Err(e) => {
            warn!(error = ?e, "casting height u32 failed, using u32::MAX");
            u32::MAX
        }
    };
    Ok(response::Query {
        key: key.into(),
        value: value.into(),
        height: Height::from(height),
        ..Default::default()
    })
}

#[non_exhaustive]
pub(crate) enum Query {
    AccountsQuery(crate::accounts::query::Request),
}

#[instrument]
fn decode_query(path: &str) -> anyhow::Result<Query> {
    let mut parts: VecDeque<&str> = path.split('/').collect();

    let Some(component) = parts.pop_front() else {
        bail!("invalid query path; missing component: {path}");
    };

    match component {
        "accounts" => {
            let request = crate::accounts::query::Request::decode(parts)
                .context("failed to decode accounts query from path parts")?;
            Ok(Query::AccountsQuery(request))
        }
        other => bail!("unknown query path: `{other}`"),
    }
}
