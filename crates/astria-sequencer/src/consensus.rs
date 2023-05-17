use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use futures::Future;
use tendermint::abci::{
    ConsensusRequest,
    ConsensusResponse,
};
use tokio::time::{
    sleep,
    Duration,
    Sleep,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

use crate::app::App;

/// Default sleep time for consensus service steps.
/// Arbitrary, used to slow down the consensus process.
pub const DEFAULT_SLEEP_TIME_SECONDS: u64 = 1;

#[derive(Clone)]
pub struct ConsensusService {
    app: Arc<App>,
}

impl ConsensusService {
    pub fn new(app: Arc<App>) -> Self {
        Self {
            app,
        }
    }

    fn deliver_tx(&self) -> ConsensusResponse {
        ConsensusResponse::DeliverTx(Default::default())
    }
}

impl Service<ConsensusRequest> for ConsensusService {
    type Error = BoxError;
    type Future = ConsensusServiceFuture;
    type Response = ConsensusResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ConsensusRequest) -> Self::Future {
        info!("got consensus request: {:?}", req);
        ConsensusServiceFuture::new(req)
    }
}

pub struct ConsensusServiceFuture {
    request: ConsensusRequest,
    sleep: Pin<Box<Sleep>>,
}

impl ConsensusServiceFuture {
    pub fn new(request: ConsensusRequest) -> Self {
        Self {
            request,
            sleep: Box::pin(sleep(Duration::from_secs(DEFAULT_SLEEP_TIME_SECONDS))),
        }
    }
}

impl Future for ConsensusServiceFuture {
    type Output = Result<ConsensusResponse, BoxError>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.sleep.as_mut().poll(_cx).is_pending() {
            return Poll::Pending;
        }

        match &self.request {
            ConsensusRequest::InitChain(_) => {
                Poll::Ready(Ok(ConsensusResponse::InitChain(Default::default())))
            }
            ConsensusRequest::BeginBlock(_) => {
                Poll::Ready(Ok(ConsensusResponse::BeginBlock(Default::default())))
            }
            ConsensusRequest::DeliverTx(_) => {
                Poll::Ready(Ok(ConsensusResponse::DeliverTx(Default::default())))
            }
            ConsensusRequest::EndBlock(_) => {
                Poll::Ready(Ok(ConsensusResponse::EndBlock(Default::default())))
            }
            ConsensusRequest::Commit => {
                Poll::Ready(Ok(ConsensusResponse::Commit(Default::default())))
            }
        }
    }
}
