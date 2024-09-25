use std::{
    pin::Pin,
    time::Duration,
};

use astria_core::{
    generated::bundle::v1alpha1::ExecuteOptimisticBlockStreamResponse,
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    Context,
};
use futures::{
    Stream,
    StreamExt as _,
};
use tokio::sync::mpsc;

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
        // TODO: move the duration value to a const or config value?
        self.blocks_to_execute_tx
            .try_send(block)
            .wrap_err("failed to send block to execute")?;

        Ok(())
    }
}

pub(crate) struct ExecutedBlockStream {
    client: Pin<Box<tonic::Streaming<ExecuteOptimisticBlockStreamResponse>>>,
}

impl ExecutedBlockStream {
    pub(crate) async fn new(
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
                client: Box::pin(executed_stream_client),
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
        unimplemented!()
    }
}
