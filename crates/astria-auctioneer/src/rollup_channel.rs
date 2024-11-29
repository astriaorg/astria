use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
    time::Duration,
};

use astria_core::generated::bundle::v1alpha1::{
    bundle_service_client::BundleServiceClient,
    BaseBlock,
    ExecuteOptimisticBlockStreamResponse,
    GetBundleStreamResponse,
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
use tonic::transport::Channel;

use crate::{
    bundle::Bundle,
    streaming_utils::restarting_stream,
};

pub(crate) fn open(endpoint: &str) -> eyre::Result<RollupChannel> {
    RollupChannel::create(endpoint)
        .wrap_err_with(|| format!("failed to create a gRPC channel to rollup at `{endpoint}`"))
}

#[derive(Clone)]
pub(crate) struct RollupChannel {
    inner: Channel,
}

impl RollupChannel {
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

    pub(crate) fn open_bundle_stream(&self) -> BundleStream {
        use astria_core::generated::bundle::v1alpha1::GetBundleStreamRequest;
        let chan = self.inner.clone();
        let inner = restarting_stream(move || {
            let chan = chan.clone();
            async move {
                let inner = BundleServiceClient::new(chan)
                    .get_bundle_stream(GetBundleStreamRequest {})
                    .await
                    .map(tonic::Response::into_inner)
                    // TODO: Don't quietly swallow this error. Provide some form of
                    // logging.
                    .ok()?;
                Some(InnerBundleStream {
                    inner,
                })
            }
        })
        .boxed();
        BundleStream {
            inner,
        }
    }

    pub(crate) fn open_execute_optimistic_block_stream(&self) -> ExecuteOptimisticBlockStream {
        use astria_core::generated::bundle::v1alpha1::{
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
                    .map(tonic::Response::into_inner)
                    // TODO: Don't quietly swallow this error. Provide some form of
                    // logging.
                    .ok()?;
                Some(InnerExecuteOptimisticBlockStream {
                    inner,
                })
            }
        })
        .boxed();

        ExecuteOptimisticBlockStream {
            incoming,
            outgoing: to_server,
        }
    }
}

pub(crate) struct BundleStream {
    inner: BoxStream<'static, eyre::Result<Bundle>>,
}

impl Stream for BundleStream {
    type Item = eyre::Result<Bundle>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

struct InnerBundleStream {
    inner: tonic::Streaming<GetBundleStreamResponse>,
}

impl Stream for InnerBundleStream {
    type Item = eyre::Result<Bundle>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(res) = ready!(self.inner.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
        };

        let raw = res
            .wrap_err("error while receiving streamed message from server")?
            .bundle
            .ok_or_else(|| {
                eyre!(
                    "message field not set: `{}.bundle`",
                    GetBundleStreamResponse::full_name()
                )
            })?;

        let bundle = Bundle::try_from_raw(raw).wrap_err_with(|| {
            format!(
                "failed to validate received message `{}`",
                astria_core::generated::bundle::v1alpha1::Bundle::full_name()
            )
        })?;

        Poll::Ready(Some(Ok(bundle)))
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
                astria_core::generated::bundle::v1alpha1::ExecuteOptimisticBlockStreamResponse::full_name(),
            )
        })?;
        Poll::Ready(Some(Ok(executed_block)))
    }
}
