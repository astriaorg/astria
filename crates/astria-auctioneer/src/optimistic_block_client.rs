use std::time::Duration;
use futures::StreamExt;

use astria_core::{
    generated::sequencerblock::optimistic::v1alpha1::{
        optimistic_block_service_client::OptimisticBlockServiceClient,
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    instrument,
    warn,
    Instrument as _,
    Span,
};
use tryhard::backoff_strategies::ExponentialBackoff;

/// Wraps the gRPC client for the Sequencer service that wraps client calls with `tryhard`.
#[derive(Debug, Clone)]
pub(crate) struct OptimisticBlockClient {
    inner: OptimisticBlockServiceClient<Channel>,
    uri: Uri,
}

impl OptimisticBlockClient {
    pub(crate) fn new(sequencer_uri: &str) -> eyre::Result<Self> {
        let uri = sequencer_uri
            .parse::<Uri>()
            .wrap_err("failed parsing provided string as Uri")?;

        // TODO: use a UDS socket instead
        let endpoint = Endpoint::from(uri.clone());
        let inner = OptimisticBlockServiceClient::new(endpoint.connect_lazy());
        Ok(Self {
            inner,
            uri,
        })
    }

    // TODO: this should probably be separated from the tryhard logic and put in an extension trait
    #[instrument(skip_all, fields(
        uri = %self.uri,
        %rollup_id,
        err,
    ))]
    pub(super) async fn get_optimistic_block_stream(
        &mut self,
        rollup_id: RollupId,
    ) -> eyre::Result<tonic::Streaming<GetOptimisticBlockStreamResponse>> {
        let span = tracing::Span::current();
        let retry_cfg = make_retry_cfg("stream optimistic blocks".into(), span);
        let client = self.inner.clone();

        let stream = tryhard::retry_fn(|| {
            let mut client = client.clone();
            let req = GetOptimisticBlockStreamRequest {
                rollup_id: Some(rollup_id.into_raw()),
            };
            async move { client.get_optimistic_block_stream(req).await }
        })
        .with_config(retry_cfg)
        .in_current_span()
        .await
        .wrap_err("failed to initialize optimistic block stream")?
        .into_inner();

        Ok(stream)
    }

    // TODO: this should probably be separated from the tryhard logic and put in an extension trait
    #[instrument(skip_all, fields(
           uri = %self.uri,
           err,
       ))]
    pub(super) async fn get_block_commitment_stream(
        &mut self,
    ) -> eyre::Result<tonic::Streaming<GetBlockCommitmentStreamResponse>> {
        let span = tracing::Span::current();
        let retry_cfg = make_retry_cfg("stream block commitments".into(), span);
        let client = self.inner.clone();

        let stream = tryhard::retry_fn(|| {
            let mut client = client.clone();
            let req = GetBlockCommitmentStreamRequest {};
            async move { client.get_block_commitment_stream(req).await }
        })
        .with_config(retry_cfg)
        .in_current_span()
        .await
        .wrap_err("failed to initialize block commitment stream")?
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
