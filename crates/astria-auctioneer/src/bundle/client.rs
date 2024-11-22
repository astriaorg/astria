use std::pin::Pin;

use astria_core::generated::bundle::v1alpha1::{
    bundle_service_client::BundleServiceClient,
    GetBundleStreamRequest,
    GetBundleStreamResponse,
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
use tonic::transport::{
    Endpoint,
    Uri,
};
use tracing::instrument;

use super::Bundle;

pub(crate) struct BundleClient {
    inner: BundleServiceClient<tonic::transport::Channel>,
    uri: Uri,
}

impl BundleClient {
    pub(crate) fn new(rollup_uri: &str) -> eyre::Result<Self> {
        let uri = rollup_uri
            .parse::<Uri>()
            .wrap_err("failed to parse rollup_grpc_endpoint")?;
        let endpoint = Endpoint::from(uri.clone());
        let client = BundleServiceClient::new(endpoint.connect_lazy());

        Ok(Self {
            inner: client,
            uri,
        })
    }

    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_bundle_stream(
        &mut self,
    ) -> eyre::Result<tonic::Streaming<GetBundleStreamResponse>> {
        let mut client = self.inner.clone();
        let stream = client
            .get_bundle_stream(GetBundleStreamRequest {})
            .await
            .wrap_err("failed to open bundle stream")?
            .into_inner();
        Ok(stream)
    }
}

pin_project_lite::pin_project! {
    pub(crate) struct BundleStream {
        #[pin]
        inner: tonic::Streaming<GetBundleStreamResponse>,
    }
}

impl BundleStream {
    pub(crate) async fn connect(rollup_grpc_endpoint: String) -> eyre::Result<Self> {
        let mut client = BundleClient::new(&rollup_grpc_endpoint)
            .wrap_err("failed to initialize bundle service client")?;
        let stream = client
            .get_bundle_stream()
            .await
            .wrap_err("failed to get bundle stream")?;

        Ok(Self {
            inner: stream,
        })
    }
}

impl Stream for BundleStream {
    type Item = eyre::Result<Bundle>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = futures::ready!(self.inner.poll_next_unpin(cx)) else {
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
