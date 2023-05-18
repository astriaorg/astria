use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures::Future;
use tendermint::abci::{
    response::{
        Echo,
        Info,
        SetOption,
    },
    InfoRequest,
    InfoResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

#[derive(Clone, Default)]
pub struct InfoService {}

impl InfoService {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct InfoServiceFuture {
    request: InfoRequest,
}

impl InfoServiceFuture {
    pub fn new(request: InfoRequest) -> Self {
        Self {
            request,
        }
    }
}

impl Future for InfoServiceFuture {
    type Output = Result<InfoResponse, BoxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.request {
            InfoRequest::Info(_) => {
                let response = InfoResponse::Info(Info {
                    version: "0.1.0".to_string(),
                    app_version: 1,
                    last_block_height: Default::default(),
                    last_block_app_hash: Default::default(),
                    data: "astria_sequencer".to_string(),
                });
                Poll::Ready(Ok(response))
            }
            InfoRequest::Echo(echo) => Poll::Ready(Ok(InfoResponse::Echo(Echo {
                message: echo.message.clone(),
            }))),
            InfoRequest::Query(_) => Poll::Ready(Ok(InfoResponse::Query(Default::default()))),
            // this was removed after v0.34
            InfoRequest::SetOption(_) => Poll::Ready(Ok(InfoResponse::SetOption(SetOption {
                code: Default::default(),
                log: "SetOption is not supported".to_string(),
                info: "SetOption is not supported".to_string(),
            }))),
        }
    }
}

impl Service<InfoRequest> for InfoService {
    type Error = BoxError;
    type Future = InfoServiceFuture;
    type Response = InfoResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: InfoRequest) -> Self::Future {
        info!("got info request: {:?}", req);
        InfoServiceFuture::new(req)
    }
}
