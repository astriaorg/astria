use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::{
    generated::astria::sequencerblock::optimistic::v1alpha1::{
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::{
        Address,
        RollupId,
    },
    sequencerblock::{
        optimistic::v1alpha1::SequencerBlockCommit,
        v1::block::FilteredSequencerBlock,
    },
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use futures::{
    stream::BoxStream,
    Future,
    Stream,
    StreamExt as _,
};
use prost::Name as _;
use tracing::{
    info_span,
    warn,
    Instrument as _,
};

use crate::streaming_utils::{
    restarting_stream,
    InstrumentedChannel,
};

pub(crate) fn open(endpoint: &str) -> eyre::Result<SequencerChannel> {
    SequencerChannel::create(endpoint)
        .wrap_err_with(|| format!("failed to create a gRPC channel to Sequencer at `{endpoint}`"))
}

#[derive(Clone)]
pub(crate) struct SequencerChannel {
    inner: InstrumentedChannel,
}

impl SequencerChannel {
    fn create(uri: &str) -> eyre::Result<Self> {
        Ok(Self {
            inner: crate::streaming_utils::make_instrumented_channel(uri)?,
        })
    }

    pub(crate) fn get_pending_nonce(
        &self,
        address: Address,
    ) -> impl Future<Output = eyre::Result<u32>> {
        use astria_core::generated::astria::sequencerblock::v1::{
            sequencer_service_client::SequencerServiceClient,
            GetPendingNonceRequest,
        };

        let mut client = SequencerServiceClient::new(self.inner.clone());
        async move {
            let nonce = client
                .get_pending_nonce(GetPendingNonceRequest {
                    address: Some(address.into_raw()),
                })
                .await
                .wrap_err("failed to fetch most recent pending nonce")?
                .into_inner()
                .inner;
            Ok(nonce)
        }
    }

    pub(crate) fn open_get_block_commitment_stream(&self) -> BlockCommitmentStream {
        use astria_core::generated::astria::sequencerblock::optimistic::v1alpha1::
            optimistic_block_service_client::OptimisticBlockServiceClient;
        let chan = self.inner.clone();
        let inner = restarting_stream(move || {
            let chan = chan.clone();
            async move {
                let inner = OptimisticBlockServiceClient::new(chan)
                    .get_block_commitment_stream(GetBlockCommitmentStreamRequest {})
                    .await
                    .wrap_err("failed to open block commitment stream")
                    .inspect_err(|error| warn!(%error))?
                    .into_inner();
                Ok(InnerBlockCommitmentStream {
                    inner,
                })
            }
            .instrument(info_span!("request block commitment stream"))
        })
        .boxed();
        BlockCommitmentStream {
            inner,
        }
    }

    pub(crate) fn open_get_proposed_block_stream(
        &self,
        rollup_id: RollupId,
    ) -> ProposedBlockStream {
        use astria_core::generated::astria::sequencerblock::optimistic::v1alpha1::{
            optimistic_block_service_client::OptimisticBlockServiceClient,
            GetOptimisticBlockStreamRequest,
        };

        let chan = self.inner.clone();
        let inner = restarting_stream(move || {
            let chan = chan.clone();
            async move {
                let mut client = OptimisticBlockServiceClient::new(chan);
                let inner = client
                    .get_optimistic_block_stream(GetOptimisticBlockStreamRequest {
                        rollup_id: Some(rollup_id.into_raw()),
                    })
                    .await
                    .wrap_err("failed to open optimistic block stream")
                    .inspect_err(|error| warn!(%error))?
                    .into_inner();
                Ok(InnerOptimisticBlockStream {
                    inner,
                })
            }
            .instrument(info_span!("request optimistic block stream"))
        })
        .boxed();
        ProposedBlockStream {
            inner,
        }
    }
}

/// A stream for receiving committed blocks from the sequencer.
pub(crate) struct BlockCommitmentStream {
    inner: BoxStream<'static, eyre::Result<SequencerBlockCommit>>,
}

impl Stream for BlockCommitmentStream {
    type Item = eyre::Result<SequencerBlockCommit>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

struct InnerBlockCommitmentStream {
    inner: tonic::Streaming<GetBlockCommitmentStreamResponse>,
}

impl Stream for InnerBlockCommitmentStream {
    type Item = eyre::Result<SequencerBlockCommit>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use astria_core::generated::astria::sequencerblock::optimistic::v1alpha1 as raw;

        let Some(res) = std::task::ready!(self.inner.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
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

        let commitment = SequencerBlockCommit::try_from_raw_ref(&raw).wrap_err_with(|| {
            format!(
                "failed to validate message `{}` received from server",
                raw::SequencerBlockCommit::full_name()
            )
        })?;

        Poll::Ready(Some(Ok(commitment)))
    }
}

pub(crate) struct ProposedBlockStream {
    inner: BoxStream<'static, eyre::Result<FilteredSequencerBlock>>,
}

impl Stream for ProposedBlockStream {
    type Item = eyre::Result<FilteredSequencerBlock>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

struct InnerOptimisticBlockStream {
    inner: tonic::Streaming<GetOptimisticBlockStreamResponse>,
}

impl Stream for InnerOptimisticBlockStream {
    type Item = eyre::Result<FilteredSequencerBlock>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(item) = ready!(self.inner.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
        };
        let raw = item
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
