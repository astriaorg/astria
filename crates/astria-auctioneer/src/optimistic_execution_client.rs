use std::time::Duration;
use futures::StreamExt;

use astria_core::{
    generated::bundle::v1alpha1::{
        optimistic_execution_service_client::OptimisticExecutionServiceClient,
        ExecuteOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    Context,
};
use tokio::sync::mpsc;
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    instrument,
    warn,
    Instrument,
    Span,
};
use tryhard::backoff_strategies::ExponentialBackoff;

use crate::block::{
    self,
    executed_stream::make_execution_requests_stream,
};

pub(crate) struct OptimisticExecutionClient {
    inner: OptimisticExecutionServiceClient<Channel>,
    uri: Uri,
}

impl OptimisticExecutionClient {
    pub(crate) fn new(rollup_uri: &str) -> eyre::Result<Self> {
        let uri = rollup_uri
            .parse::<Uri>()
            .wrap_err("failed parsing optimistic execution uri")?;

        // TODO: use UDS socket
        let endpoint = Endpoint::from(uri.clone());
        let inner = OptimisticExecutionServiceClient::new(endpoint.connect_lazy());

        Ok(Self {
            inner,
            uri,
        })
    }

    #[instrument(skip_all, fields(
        uri = %self.uri,
        %rollup_id,
        err,
    ))]
    pub(crate) async fn execute_optimistic_block_stream(
        &mut self,
        rollup_id: RollupId,
    ) -> eyre::Result<(
        tonic::Streaming<ExecuteOptimisticBlockStreamResponse>,
        mpsc::Sender<block::Optimistic>,
    )> {
        let span = tracing::Span::current();
        let retry_cfg = make_retry_cfg("execute optimistic blocks".into(), span);
        let client = self.inner.clone();

        let (stream, opt_tx) = tryhard::retry_fn(|| {
            let mut client = client.clone();

            let (blocks_to_execute_tx, requests) = make_execution_requests_stream(rollup_id);

            async move {
                let stream = client.execute_optimistic_block_stream(requests).await?;
                Ok((stream, blocks_to_execute_tx))
            }
        })
        .with_config(retry_cfg)
        .in_current_span()
        .await
        .wrap_err("failed to initialize optimistic execution stream")?;

        Ok((stream.into_inner(), opt_tx))
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
