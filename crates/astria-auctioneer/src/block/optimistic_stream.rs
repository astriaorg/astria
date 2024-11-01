use std::pin::Pin;

use astria_core::{
    generated::sequencerblock::optimisticblock::v1alpha1::GetOptimisticBlockStreamResponse,
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    Context,
    OptionExt,
};
use futures::{
    Stream,
    StreamExt as _,
};

use super::Optimistic;
use crate::optimistic_block_client::OptimisticBlockClient;

/// A stream for receiving optimistic blocks from the sequencer.
pub(crate) struct OptimisticBlockStream {
    client: Pin<Box<tonic::Streaming<GetOptimisticBlockStreamResponse>>>,
}

impl OptimisticBlockStream {
    pub(crate) async fn connect(
        rollup_id: RollupId,
        mut sequencer_client: OptimisticBlockClient,
    ) -> eyre::Result<OptimisticBlockStream> {
        let optimistic_stream_client = sequencer_client
            .get_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("failed to stream optimistic blocks")?;

        Ok(OptimisticBlockStream {
            // client,
            client: Box::pin(optimistic_stream_client),
        })
    }
}

impl Stream for OptimisticBlockStream {
    type Item = eyre::Result<Optimistic>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let raw = futures::ready!(self.client.poll_next_unpin(cx))
            // TODO: better error messages here
            .ok_or_eyre("stream has been closed")?
            .wrap_err("received gRPC error")?
            .block
            .ok_or_eyre(
                "optimsitic block stream response did not contain filtered sequencer block",
            )?;

        let optimistic_block =
            Optimistic::try_from_raw(raw).wrap_err("failed to parse raw to Optimistic")?;

        std::task::Poll::Ready(Some(Ok(optimistic_block)))
    }
}
