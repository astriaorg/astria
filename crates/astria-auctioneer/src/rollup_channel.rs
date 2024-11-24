use std::time::Duration;

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
    Stream,
    StreamExt,
};
use prost::Name as _;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

use crate::bundle::Bundle;

pub(crate) fn open(endpoint: &str) -> eyre::Result<RollupChannel> {
    RollupChannel::create(&endpoint)
        .wrap_err_with(|| format!("failed to create a gRPC channel to rollup at `{endpoint}`"))
}

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

    pub(crate) async fn open_bundle_stream(&self) -> eyre::Result<BundleStream> {
        use astria_core::generated::bundle::v1alpha1::GetBundleStreamRequest;
        let inner = BundleServiceClient::new(self.inner.clone())
            .get_bundle_stream(GetBundleStreamRequest {})
            .await
            .wrap_err("failed to open get bundle stream")?
            .into_inner();
        Ok(BundleStream {
            inner,
        })
    }

    pub(crate) async fn open_execute_optimistic_block_stream(
        &self,
    ) -> eyre::Result<ExecuteOptimisticBlockStream> {
        use astria_core::generated::bundle::v1alpha1::{
            optimistic_execution_service_client::OptimisticExecutionServiceClient,
            ExecuteOptimisticBlockStreamRequest,
        };

        let (to_server_tx, to_server_rx) = mpsc::channel(16);
        let out_stream = ReceiverStream::new(to_server_rx).map(|base_block| {
            ExecuteOptimisticBlockStreamRequest {
                base_block: Some(base_block),
            }
        });
        let from_server = OptimisticExecutionServiceClient::new(self.inner.clone())
            .execute_optimistic_block_stream(out_stream)
            .await
            .wrap_err("failed to open execute optimistic block stream")?
            .into_inner();

        Ok(ExecuteOptimisticBlockStream {
            incoming: from_server,
            outgoing: to_server_tx,
        })
    }
}

pub(crate) struct BundleStream {
    inner: tonic::Streaming<GetBundleStreamResponse>,
}

impl Stream for BundleStream {
    type Item = eyre::Result<Bundle>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = std::task::ready!(self.inner.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("failed receiving streamed message from server")?
            .bundle
            .ok_or_else(|| {
                eyre!(
                    "message field not set: `{}.bundle`",
                    GetBundleStreamResponse::full_name()
                )
            })?;

        let bundle = Bundle::try_from_raw(raw).wrap_err_with(|| {
            format!(
                "failed to validate received `{}`",
                astria_core::generated::bundle::v1alpha1::Bundle::full_name()
            )
        })?;

        std::task::Poll::Ready(Some(Ok(bundle)))
    }
}

pub(crate) struct ExecuteOptimisticBlockStream {
    incoming: tonic::Streaming<ExecuteOptimisticBlockStreamResponse>,
    outgoing: mpsc::Sender<BaseBlock>,
}

impl ExecuteOptimisticBlockStream {
    /// Immediately sends `base_block` to the connected server. Fails if
    /// the channel is full.
    // NOTE: just leak the tokio mpsc error for now. It's crate private anyway
    // and we'd just end up wrapping the same variants.
    pub(crate) fn try_send(
        &mut self,
        base_block: BaseBlock,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<BaseBlock>> {
        self.outgoing.try_send(base_block)
    }
}

impl Stream for ExecuteOptimisticBlockStream {
    type Item = eyre::Result<crate::block::Executed>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(message) = std::task::ready!(self.incoming.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let message = message.wrap_err("failed receiving message over stream")?;
        let executed_block = crate::block::Executed::try_from_raw(message).wrap_err_with(|| {
            format!(
                "failed to validate `{}`",
                astria_core::generated::bundle::v1alpha1::ExecuteOptimisticBlockStreamResponse::full_name(),
            )
        })?;
        std::task::Poll::Ready(Some(Ok(executed_block)))
    }
}
