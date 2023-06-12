use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures::Future;
use tendermint::abci::{
    SnapshotRequest,
    SnapshotResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::{
    instrument,
    instrument::Instrumented,
    Instrument as _,
};

#[derive(Clone, Default)]
pub(crate) struct Snapshot {}

impl Snapshot {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Service<SnapshotRequest> for Snapshot {
    type Error = BoxError;
    type Future = Instrumented<SnapshotFuture>;
    type Response = SnapshotResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[instrument(name = "Snapshot::call", skip(self))]
    fn call(&mut self, req: SnapshotRequest) -> Self::Future {
        SnapshotFuture::new(req).in_current_span()
    }
}

pub(crate) struct SnapshotFuture {
    request: SnapshotRequest,
}

impl SnapshotFuture {
    fn new(request: SnapshotRequest) -> Self {
        Self {
            request,
        }
    }
}

impl Future for SnapshotFuture {
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
