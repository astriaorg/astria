use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::generated::astria::{
    auction::v1alpha1::GetBidStreamResponse,
    optimistic_execution::v1alpha1::{
        BaseBlock,
        ExecuteOptimisticBlockStreamResponse,
    },
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use futures::{
    stream::BoxStream,
    Stream,
    StreamExt,
};
use prost::Name as _;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{
    info_span,
    warn,
    Instrument as _,
};

use crate::{
    bid::Bid,
    streaming_utils::{
        make_instrumented_channel,
        restarting_stream,
        InstrumentedChannel,
    },
};

pub(crate) fn open(endpoint: &str) -> eyre::Result<RollupChannel> {
    RollupChannel::create(endpoint)
        .wrap_err_with(|| format!("failed to create a gRPC channel to rollup at `{endpoint}`"))
}

#[derive(Clone)]
pub(crate) struct RollupChannel {
    inner: InstrumentedChannel,
}

impl RollupChannel {
    fn create(uri: &str) -> eyre::Result<Self> {
        Ok(Self {
            inner: make_instrumented_channel(uri)?,
        })
    }

    pub(crate) fn open_bid_stream(&self) -> BidStream {
        use astria_core::generated::astria::auction::v1alpha1::{
            auction_service_client::AuctionServiceClient,
            GetBidStreamRequest,
        };
        let chan = self.inner.clone();
        let inner = restarting_stream(move || {
            let chan = chan.clone();
            async move {
                let inner = AuctionServiceClient::new(chan)
                    .get_bid_stream(GetBidStreamRequest {})
                    .await
                    .wrap_err("failed to open bid stream")
                    .inspect_err(|error| warn!(%error))?
                    .into_inner();
                Ok(InnerBidStream {
                    inner,
                })
            }
            .instrument(info_span!("request bid stream"))
        })
        .boxed();
        BidStream {
            inner,
        }
    }

    pub(crate) fn open_execute_optimistic_block_stream(&self) -> ExecuteOptimisticBlockStream {
        use astria_core::generated::astria::optimistic_execution::v1alpha1::{
            optimistic_execution_service_client::OptimisticExecutionServiceClient,
            ExecuteOptimisticBlockStreamRequest,
        };

        // NOTE: this implementation uses a broadcast channel instead of an mpsc because
        // one can get new readers by using Sender::subscribe. This is important for the
        // restart mechanism. The mpsc channel (or rather the tokio stream ReceiverStream wrapper)
        // would need something ugly like a Arc<tokio::Mutex<ReceiverStream>>, but where
        // we'd need to also implement Stream for some wrapper around it.... It's a mess.
        let (to_server, _) = broadcast::channel(16);
        let chan = self.inner.clone();
        let to_server_2 = to_server.clone();
        let incoming = restarting_stream(move || {
            let chan = chan.clone();
            let out_stream = BroadcastStream::new(to_server_2.subscribe())
                // TODO: emit some kind of event when the stream actually starts
                // lagging behind instead of quietly discarding the issue.
                .filter_map(|maybe_lagged| std::future::ready(maybe_lagged.ok()))
                .map(|base_block| ExecuteOptimisticBlockStreamRequest {
                    base_block: Some(base_block),
                });

            async move {
                let inner = OptimisticExecutionServiceClient::new(chan)
                    .execute_optimistic_block_stream(out_stream)
                    .await
                    .wrap_err("failed to open execute optimistic block stream")
                    .inspect_err(|error| warn!(%error))?
                    .into_inner();
                Ok(InnerExecuteOptimisticBlockStream {
                    inner,
                })
            }
            .instrument(info_span!("request execute optimistic block stream"))
        })
        .boxed();

        ExecuteOptimisticBlockStream {
            incoming,
            outgoing: to_server,
        }
    }
}

pub(crate) struct BidStream {
    inner: BoxStream<'static, eyre::Result<Bid>>,
}

impl Stream for BidStream {
    type Item = eyre::Result<Bid>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

struct InnerBidStream {
    inner: tonic::Streaming<GetBidStreamResponse>,
}

impl Stream for InnerBidStream {
    type Item = eyre::Result<Bid>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(res) = ready!(self.inner.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
        };

        let raw = res
            .wrap_err("error while receiving streamed message from server")?
            .bid
            .ok_or_else(|| {
                eyre!(
                    "message field not set: `{}.bid`",
                    GetBidStreamResponse::full_name()
                )
            })?;

        let bid = Bid::try_from_raw(raw).wrap_err_with(|| {
            format!(
                "failed to validate received message `{}`",
                astria_core::generated::astria::auction::v1alpha1::Bid::full_name()
            )
        })?;

        Poll::Ready(Some(Ok(bid)))
    }
}

pub(crate) struct ExecuteOptimisticBlockStream {
    incoming: BoxStream<'static, eyre::Result<crate::block::Executed>>,
    outgoing: broadcast::Sender<BaseBlock>,
}

impl ExecuteOptimisticBlockStream {
    /// Immediately sends `base_block` to the connected server. Fails if
    /// the channel is full.
    // NOTE: just leak the tokio error for now. It's crate private anyway
    // and we'd just end up wrapping the same variants.
    pub(crate) fn try_send(
        &mut self,
        base_block: BaseBlock,
    ) -> Result<(), broadcast::error::SendError<BaseBlock>> {
        self.outgoing.send(base_block).map(|_| ())
    }
}

impl Stream for ExecuteOptimisticBlockStream {
    type Item = eyre::Result<crate::block::Executed>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.incoming.poll_next_unpin(cx)
    }
}

pub(crate) struct InnerExecuteOptimisticBlockStream {
    inner: tonic::Streaming<ExecuteOptimisticBlockStreamResponse>,
}

impl Stream for InnerExecuteOptimisticBlockStream {
    type Item = eyre::Result<crate::block::Executed>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(message) = ready!(self.inner.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
        };

        let message = message.wrap_err("failed receiving message over stream")?;
        let executed_block = crate::block::Executed::try_from_raw(message).wrap_err_with(|| {
            format!(
                "failed to validate `{}`",
                astria_core::generated::astria::optimistic_execution::v1alpha1::ExecuteOptimisticBlockStreamResponse::full_name(),
            )
        })?;
        Poll::Ready(Some(Ok(executed_block)))
    }
}
