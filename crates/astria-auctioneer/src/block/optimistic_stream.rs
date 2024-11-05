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
use telemetry::display::base64;
use tracing::debug;

use super::Optimistic;
use crate::optimistic_block_client::OptimisticBlockClient;

/// A stream for receiving optimistic blocks from the sequencer.
// TODO: pin project these instead
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
        // TODO: return none when stream is closed
        let rsp = match futures::ready!(self.client.poll_next_unpin(cx)) {
            Some(raw) => raw,
            None => return std::task::Poll::Ready(None),
        };

        // TODO: filter_map on these errors
        let raw = rsp.wrap_err("received gRPC error")?.block.ok_or_eyre(
            "optimsitic block stream response did not contain filtered sequencer block",
        )?;

        let optimistic_block =
            Optimistic::try_from_raw(raw).wrap_err("failed to parse raw to Optimistic")?;

        debug!(
            optimistic_block.sequencer_block_hash = %base64(optimistic_block.sequencer_block_hash()),
            "received optimistic block from sequencer"
        );

        std::task::Poll::Ready(Some(Ok(optimistic_block)))
    }
}
