use std::pin::Pin;

use astria_core::{
    generated::bundle::v1alpha1::{
        ExecuteOptimisticBlockStreamRequest,
        ExecuteOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::{
    Stream,
    StreamExt,
};
use pin_project_lite::pin_project;
use telemetry::display::base64;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{
    debug,
    error,
};

use super::{
    Executed,
    Optimistic,
};
use crate::optimistic_execution_client::OptimisticExecutionClient;

pub(crate) struct Handle {
    blocks_to_execute_tx: mpsc::Sender<Optimistic>,
}

impl Handle {
    pub(crate) fn try_send_block_to_execute(&mut self, block: Optimistic) -> eyre::Result<()> {
        self.blocks_to_execute_tx
            .try_send(block)
            .wrap_err("failed to send block to execute")?;

        Ok(())
    }
}

pin_project! {
    /// A stream for receiving optimistic execution results from the rollup node.
    pub(crate) struct ExecutedBlockStream {
        #[pin]
        client: tonic::Streaming<ExecuteOptimisticBlockStreamResponse>,
    }
}

impl ExecutedBlockStream {
    pub(crate) async fn connect(
        rollup_id: RollupId,
        rollup_grpc_endpoint: String,
    ) -> eyre::Result<(Handle, ExecutedBlockStream)> {
        let mut optimistic_execution_client = OptimisticExecutionClient::new(&rollup_grpc_endpoint)
            .wrap_err("failed to initialize optimistic execution client")?;
        let (executed_stream_client, blocks_to_execute_tx) = optimistic_execution_client
            .execute_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("failed to stream executed blocks")?;

        Ok((
            Handle {
                blocks_to_execute_tx,
            },
            Self {
                client: executed_stream_client,
            },
        ))
    }
}

impl Stream for ExecutedBlockStream {
    type Item = eyre::Result<Executed>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let res = match futures::ready!(self.client.poll_next_unpin(cx)) {
            Some(res) => res,
            None => return std::task::Poll::Ready(None),
        };

        let raw = res.wrap_err("received gRPC Error")?;

        let executed_block =
            Executed::try_from_raw(raw).wrap_err("failed to parse raw to Executed")?;

        debug!(
            executed_block.rollup_block_hash = %base64(executed_block.rollup_block_hash()),
            executed_block.sequencer_block_hash = %base64(executed_block.sequencer_block_hash()),
            "received block execution result"
        );

        std::task::Poll::Ready(Some(Ok(executed_block)))
    }
}

pub(crate) fn make_execution_requests_stream(
    rollup_id: RollupId,
) -> (
    mpsc::Sender<Optimistic>,
    impl tonic::IntoStreamingRequest<Message = ExecuteOptimisticBlockStreamRequest>,
) {
    // TODO: should this capacity be a config instead of a magic number? OPTIMISTIC_REORG_BUFFER?
    // TODO: add a metric so we can see if that becomes a problem
    let (blocks_to_execute_tx, blocks_to_execute_rx) = mpsc::channel(16);
    let blocks_to_execute_stream_rx = ReceiverStream::new(blocks_to_execute_rx);

    let requests = blocks_to_execute_stream_rx.filter_map(move |block: Optimistic| async move {
        let base_block = block
            .try_into_base_block(rollup_id)
            .wrap_err("failed to create BaseBlock from FilteredSequencerBlock");

        // skip blocks which fail to decode the transactions?
        match base_block {
            Ok(base_block) => Some(ExecuteOptimisticBlockStreamRequest {
                base_block: Some(base_block),
            }),
            Err(e) => {
                error!(error = %e, "skipping execution of invalid block");

                None
            }
        }
    });

    (blocks_to_execute_tx, requests)
}
