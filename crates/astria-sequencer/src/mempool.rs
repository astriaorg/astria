use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tendermint::abci::{MempoolRequest, MempoolResponse};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

#[derive(Clone)]
pub struct MempoolService {}

impl MempoolService {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct MempoolServiceFuture {
    request: MempoolRequest,
}

impl MempoolServiceFuture {
    pub fn new(request: MempoolRequest) -> Self {
        Self { request }
    }
}

impl Future for MempoolServiceFuture {
    type Output = Result<MempoolResponse, BoxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.request {
            MempoolRequest::CheckTx(_) => {
                Poll::Ready(Ok(MempoolResponse::CheckTx(Default::default())))
            }
        }
    }
}

impl Service<MempoolRequest> for MempoolService {
    type Response = MempoolResponse;
    type Error = BoxError;
    type Future = MempoolServiceFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        info!("got mempool request: {:?}", req);
        MempoolServiceFuture::new(req)
    }
}
