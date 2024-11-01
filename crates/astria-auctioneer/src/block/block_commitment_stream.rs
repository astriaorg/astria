use std::pin::Pin;

use astria_core::generated::sequencerblock::optimisticblock::v1alpha1::GetBlockCommitmentStreamResponse;
use astria_eyre::eyre::{
    self,
    Context,
    OptionExt,
};
use futures::{
    Stream,
    StreamExt as _,
};

use super::Commitment;
use crate::optimistic_block_client::OptimisticBlockClient;

/// A stream for receiving committed blocks from the sequencer.
pub(crate) struct BlockCommitmentStream {
    client: Pin<Box<tonic::Streaming<GetBlockCommitmentStreamResponse>>>,
}

impl BlockCommitmentStream {
    pub(crate) async fn connect(mut sequencer_client: OptimisticBlockClient) -> eyre::Result<Self> {
        let committed_stream_client = sequencer_client
            .get_block_commitment_stream()
            .await
            .wrap_err("failed to stream block commitments")?;

        Ok(Self {
            client: Box::pin(committed_stream_client),
        })
    }
}

impl Stream for BlockCommitmentStream {
    type Item = eyre::Result<Commitment>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let raw = futures::ready!(self.client.poll_next_unpin(cx))
            .ok_or_eyre("stream has been closed")?
            .wrap_err("received gRPC error")?
            .commitment
            .ok_or_eyre("block commitment stream response did not contain block commitment")?;

        let commitment =
            Commitment::try_from_raw(raw).wrap_err("failed to parse raw to BlockCommitment")?;

        std::task::Poll::Ready(Some(Ok(commitment)))
    }
}
