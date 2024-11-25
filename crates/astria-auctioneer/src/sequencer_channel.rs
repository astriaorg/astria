use std::{
    pin::Pin,
    time::Duration,
};

use astria_core::{
    generated::sequencerblock::optimisticblock::v1alpha1::{
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::block::FilteredSequencerBlock,
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use futures::{
    Stream,
    StreamExt as _,
};
use prost::Name;
use tonic::transport::Channel;

use crate::block::Commitment;

pub(crate) fn open(endpoint: &str) -> eyre::Result<SequencerChannel> {
    SequencerChannel::create(&endpoint)
        .wrap_err_with(|| format!("failed to create a gRPC channel to Sequencer at `{endpoint}`"))
}

#[derive(Clone)]
pub(crate) struct SequencerChannel {
    inner: Channel,
}

impl SequencerChannel {
    fn create(uri: &str) -> eyre::Result<Self> {
        let channel = Channel::from_shared(uri.to_string())
            .wrap_err("failed to open a channel to the provided uri")?
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(2))
            .connect_lazy();

        Ok(Self {
            inner: channel,
        })
    }

    pub(crate) async fn open_get_block_commitment_stream(
        &self,
    ) -> eyre::Result<BlockCommitmentStream> {
        use astria_core::generated::sequencerblock::optimisticblock::v1alpha1::
            optimistic_block_service_client::OptimisticBlockServiceClient;
        let mut client = OptimisticBlockServiceClient::new(self.inner.clone());
        let stream = client
            .get_block_commitment_stream(GetBlockCommitmentStreamRequest {})
            .await
            .wrap_err("failed to open block commitment stream")?
            .into_inner();
        Ok(BlockCommitmentStream::new(stream))
    }

    pub(crate) async fn open_get_optimistic_block_stream(
        &self,
        rollup_id: RollupId,
    ) -> eyre::Result<OptimisticBlockStream> {
        use astria_core::generated::sequencerblock::optimisticblock::v1alpha1::{
            optimistic_block_service_client::OptimisticBlockServiceClient,
            GetOptimisticBlockStreamRequest,
        };
        let mut client = OptimisticBlockServiceClient::new(self.inner.clone());
        let stream = client
            .get_optimistic_block_stream(GetOptimisticBlockStreamRequest {
                rollup_id: Some(rollup_id.into_raw()),
            })
            .await
            .wrap_err("failed to open optimistic block stream")?
            .into_inner();
        Ok(OptimisticBlockStream::new(stream))
    }
}

/// A stream for receiving committed blocks from the sequencer.
pub(crate) struct BlockCommitmentStream {
    inner: tonic::Streaming<GetBlockCommitmentStreamResponse>,
}

impl BlockCommitmentStream {
    fn new(inner: tonic::Streaming<GetBlockCommitmentStreamResponse>) -> Self {
        Self {
            inner,
        }
    }
}

impl Stream for BlockCommitmentStream {
    type Item = eyre::Result<Commitment>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = std::task::ready!(self.inner.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("failed receiving message over stream")?
            .commitment
            .ok_or_else(|| {
                eyre!(
                    "expected field `{}.commitment` was not set",
                    GetBlockCommitmentStreamResponse::full_name()
                )
            })?;

        use astria_core::generated::sequencerblock::optimisticblock::v1alpha1 as raw;
        let commitment = Commitment::try_from_raw(&raw).wrap_err_with(|| {
            format!(
                "failed to validate message `{}` received from server",
                raw::SequencerBlockCommit::full_name()
            )
        })?;

        std::task::Poll::Ready(Some(Ok(commitment)))
    }
}

pub(crate) struct OptimisticBlockStream {
    inner: tonic::Streaming<GetOptimisticBlockStreamResponse>,
}

impl OptimisticBlockStream {
    fn new(inner: tonic::Streaming<GetOptimisticBlockStreamResponse>) -> Self {
        Self {
            inner,
        }
    }
}

impl Stream for OptimisticBlockStream {
    type Item = eyre::Result<FilteredSequencerBlock>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = futures::ready!(self.inner.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("failed receiving message over stream")?
            .block
            .ok_or_else(|| {
                eyre!(
                    "expected field `{}.block` was not set",
                    GetOptimisticBlockStreamRequest::full_name()
                )
            })?;

        let optimistic_block = FilteredSequencerBlock::try_from_raw(raw).wrap_err_with(|| {
            format!(
                "failed to validate `{}`",
                FilteredSequencerBlock::full_name()
            )
        })?;

        std::task::Poll::Ready(Some(Ok(optimistic_block)))
    }
}
