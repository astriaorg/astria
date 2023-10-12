use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures::{
    Future,
    FutureExt,
};
use penumbra_tower_trace::v037::RequestExt as _;
use tendermint::v0_37::abci::{
    response::{
        ApplySnapshotChunk,
        ListSnapshots,
        LoadSnapshotChunk,
        OfferSnapshot,
    },
    SnapshotRequest,
    SnapshotResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument as _;

#[derive(Clone, Default)]
pub(crate) struct Snapshot;

impl Service<SnapshotRequest> for Snapshot {
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<SnapshotResponse, BoxError>> + Send + 'static>>;
    type Response = SnapshotResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: SnapshotRequest) -> Self::Future {
        let span = req.create_span();
        async move {
            Ok(match req {
                SnapshotRequest::ListSnapshots => {
                    SnapshotResponse::ListSnapshots(ListSnapshots::default())
                }

                SnapshotRequest::OfferSnapshot(_) => {
                    SnapshotResponse::OfferSnapshot(OfferSnapshot::default())
                }

                SnapshotRequest::LoadSnapshotChunk(_) => {
                    SnapshotResponse::LoadSnapshotChunk(LoadSnapshotChunk::default())
                }

                SnapshotRequest::ApplySnapshotChunk(_) => {
                    SnapshotResponse::ApplySnapshotChunk(ApplySnapshotChunk::default())
                }
            })
        }
        .instrument(span)
        .boxed()
    }
}
