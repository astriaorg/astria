use std::pin::Pin;

use astria_core::generated::sequencerblock::v1alpha1::StreamOptimisticBlockResponse;
use astria_eyre::eyre::{
    self,
};
use futures::{
    Stream,
    StreamExt as _,
};

use super::block;

pub(crate) struct OptimisticBlockStream {
    // TODO: does this need to be pinned?
    // client: tonic::Streaming<StreamOptimisticBlockResponse>,
    client: Pin<Box<dyn Stream<Item = Result<StreamOptimisticBlockResponse, tonic::Status>>>>,
}

impl OptimisticBlockStream {
    pub(crate) fn new(client: tonic::Streaming<StreamOptimisticBlockResponse>) -> Self {
        Self {
            // client,
            client: Box::pin(client),
        }
    }
}

// this should have a stream impl that produces `block::Optimistic` structs

impl Stream for OptimisticBlockStream {
    type Item = eyre::Result<block::Optimistic>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        // is this the correct way to use use tonic::Streaming here
        // TODO: don't unwrap unwrap the streams
        let raw = futures::ready!(self.client.poll_next_unpin(cx))
            .unwrap()
            .unwrap();
        // convert raw to block::Optimistic
        let opt = block::Optimistic::from_raw(raw);
        std::task::Poll::Ready(Some(Ok(opt)))
    }
}
