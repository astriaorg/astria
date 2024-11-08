use std::pin::Pin;

use astria_core::generated::sequencerblock::optimisticblock::v1alpha1::GetBlockCommitmentStreamResponse;
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

use super::Commitment;
use crate::optimistic_block_client::OptimisticBlockClient;

pin_project! {
    /// A stream for receiving committed blocks from the sequencer.
    pub(crate) struct BlockCommitmentStream {
        #[pin]
        client: tonic::Streaming<GetBlockCommitmentStreamResponse>,
    }
}

impl BlockCommitmentStream {
    pub(crate) async fn connect(mut sequencer_client: OptimisticBlockClient) -> eyre::Result<Self> {
        let commitment_stream_client = sequencer_client
            .get_block_commitment_stream()
            .await
            .wrap_err("failed to stream block commitments")?;

        Ok(Self {
            client: commitment_stream_client,
        })
    }
}

impl Stream for BlockCommitmentStream {
    type Item = eyre::Result<Commitment>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = futures::ready!(self.client.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("received gRPC error")?
            .commitment
            .ok_or_eyre("response did not contain block commitment")?;

        let commitment =
            Commitment::try_from_raw(&raw).wrap_err("failed to parse raw to BlockCommitment")?;

        debug!(block_commitment.sequencer_block_hash = %base64(&commitment.sequencer_block_hash()), "received block commitment");

        std::task::Poll::Ready(Some(Ok(commitment)))
    }
}
