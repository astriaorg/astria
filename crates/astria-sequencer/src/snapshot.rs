use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tendermint::abci::{SnapshotRequest, SnapshotResponse};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

#[derive(Clone)]
pub struct SnapshotService {}

impl SnapshotService {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct SnapshotServiceFuture {
    request: SnapshotRequest,
}

impl SnapshotServiceFuture {
    pub fn new(request: SnapshotRequest) -> Self {
        Self { request }
    }
}

impl Future for SnapshotServiceFuture {
    type Output = Result<SnapshotResponse, BoxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.request {
            SnapshotRequest::ListSnapshots => {
                Poll::Ready(Ok(SnapshotResponse::ListSnapshots(Default::default())))
            }
            SnapshotRequest::OfferSnapshot(_) => {
                Poll::Ready(Ok(SnapshotResponse::OfferSnapshot(Default::default())))
            }
            SnapshotRequest::LoadSnapshotChunk(_) => {
                Poll::Ready(Ok(SnapshotResponse::LoadSnapshotChunk(Default::default())))
            }
            SnapshotRequest::ApplySnapshotChunk(_) => {
                Poll::Ready(Ok(SnapshotResponse::ApplySnapshotChunk(Default::default())))
            }
        }
    }
}

impl Service<SnapshotRequest> for SnapshotService {
    type Response = SnapshotResponse;
    type Error = BoxError;
    type Future = SnapshotServiceFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: SnapshotRequest) -> Self::Future {
        info!("got snapshot request: {:?}", req);
        SnapshotServiceFuture::new(req)
    }
}
