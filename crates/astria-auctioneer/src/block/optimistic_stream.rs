use std::pin::Pin;

use astria_core::{
    generated::sequencerblock::optimistic::v1alpha1::GetOptimisticBlockStreamResponse,
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    Context,
    OptionExt as _,
};
use futures::{
    Stream,
    StreamExt as _,
};
use pin_project_lite::pin_project;
use telemetry::display::base64;
use tracing::debug;

use super::{
    executed_stream,
    Optimistic,
};
use crate::optimistic_block_client::OptimisticBlockClient;

pin_project! {
    /// A stream for receiving optimistic blocks from the sequencer.
    /// Blocks received optimistically will be checked for validity and a clone will
    /// be sent to the rollup's optimistic execution serivce before returning them
    /// for further processing.
    ///
    /// ## Backpressure
    /// While blocks are forwarded using an `mpsc` channel, we only receive incoming
    /// optimistic blocks from the sequencer when CometBFT proposals are processed.
    /// Multiple optimsitic blocks will be received in a short amount of time only in
    /// the event that CometBFT receives multiple proposals within a single block's slot.
    /// We assume this is relatively rare and that under normal operations a block will
    /// be sent optimistically once per slot (~2 seconds).
    pub(crate) struct OptimisticBlockStream {
        #[pin]
        client: tonic::Streaming<GetOptimisticBlockStreamResponse>,
        #[pin]
        execution_handle: executed_stream::Handle,
    }
}

impl OptimisticBlockStream {
    pub(crate) async fn connect(
        rollup_id: RollupId,
        mut sequencer_client: OptimisticBlockClient,
        execution_handle: executed_stream::Handle,
    ) -> eyre::Result<OptimisticBlockStream> {
        let optimistic_stream_client = sequencer_client
            .get_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("failed to stream optimistic blocks")?;

        Ok(OptimisticBlockStream {
            client: optimistic_stream_client,
            execution_handle,
        })
    }
}

impl Stream for OptimisticBlockStream {
    type Item = eyre::Result<Optimistic>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = futures::ready!(self.client.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("received gRPC error")?
            .block
            .ok_or_eyre("response did not contain filtered sequencer block")?;

        let optimistic_block =
            Optimistic::try_from_raw(raw).wrap_err("failed to parse raw to Optimistic")?;

        debug!(
            optimistic_block.sequencer_block_hash = %base64(optimistic_block.sequencer_block_hash()),
            "received optimistic block from sequencer"
        );

        // forward the optimistic block to the rollup's optimistic execution server
        self.execution_handle
            .try_send_block_to_execute(optimistic_block.clone())
            .wrap_err("failed to send optimistic block for execution")?;

        std::task::Poll::Ready(Some(Ok(optimistic_block)))
    }
}
