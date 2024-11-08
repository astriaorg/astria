use std::{
    pin::Pin,
    time::Duration,
};

use astria_core::generated::bundle::v1alpha1::{
    bundle_service_client::BundleServiceClient,
    GetBundleStreamRequest,
    GetBundleStreamResponse,
};
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
use axum::http::Uri;
use futures::{
    Stream,
    StreamExt,
};
use tonic::transport::Endpoint;
use tracing::{
    instrument,
    warn,
    Instrument,
    Span,
};
use tryhard::backoff_strategies::ExponentialBackoff;

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
        let span = tracing::Span::current();
        let retry_cfg = make_retry_cfg("get bundle stream".into(), span);
        let client = self.inner.clone();

        let stream = tryhard::retry_fn(|| {
            let mut client = client.clone();
            async move { client.get_bundle_stream(GetBundleStreamRequest {}).await }
        })
        .with_config(retry_cfg)
        .in_current_span()
        .await
        .wrap_err("failed to get bundle stream")?
        .into_inner();

        Ok(stream)
    }
}

fn make_retry_cfg(
    msg: String,
    span: Span,
) -> tryhard::RetryFutureConfig<
    ExponentialBackoff,
    impl Fn(u32, Option<Duration>, &tonic::Status) -> futures::future::Ready<()>,
> {
    tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(2))
        .on_retry(
            move |attempt: u32, next_delay: Option<Duration>, error: &tonic::Status| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: &span,
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to {msg} failed; retrying after backoff",
                );
                futures::future::ready(())
            },
        )
}

pub(crate) struct BundleStream {
    client: Pin<Box<tonic::Streaming<GetBundleStreamResponse>>>,
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
            client: Box::pin(stream),
        })
    }
}

impl Stream for BundleStream {
    type Item = eyre::Result<Bundle>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(res) = futures::ready!(self.client.poll_next_unpin(cx)) else {
            return std::task::Poll::Ready(None);
        };

        let raw = res
            .wrap_err("received gRPC error")?
            .bundle
            .ok_or_eyre("bundle stream response did not contain bundle")?;

        let bundle = Bundle::try_from_raw(raw).wrap_err("failed to parse raw Bundle")?;

        std::task::Poll::Ready(Some(Ok(bundle)))
    }
}
