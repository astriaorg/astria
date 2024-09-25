use std::time::Duration;

use astria_core::{
    generated::bundle::v1alpha1::{
        optimistic_execution_service_client::OptimisticExecutionServiceClient,
        ExecuteOptimisticBlockStreamRequest,
        ExecuteOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    Context,
};
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::{
    wrappers::ReceiverStream,
    StreamExt as _,
};
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    warn,
    Instrument,
    Span,
};
use tryhard::backoff_strategies::ExponentialBackoff;

use crate::block;

struct OptimisitcBlocksToExecuteStream {
    opts: ReceiverStream<block::Optimistic>,
}

impl Stream for OptimisitcBlocksToExecuteStream {
    type Item = eyre::Result<block::Optimistic>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        // opts.next
        unimplemented!("get an opt block if possible. otherwise return pending?");
    }
}

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

            // create request stream
            let (opts_to_exec_tx, opts_to_exec_rx) = mpsc::channel(16);
            let opts = ReceiverStream::new(opts_to_exec_rx);
            let rollup_id = rollup_id.clone();

            let requests = opts.map(move |opt: block::Optimistic| {
                let base_block = opt
                    .try_into_base_block(rollup_id)
                    .wrap_err("failed to create BaseBlock from filtered_sequencer_block")
                    // TODO: get rid of this unwrap so we can handle blocks with no transactions.
                    // - instead of opts.map(), i should onyl create exec requests for blocks with
                    //   transactions in them.
                    // - moving this into a domain specific stream will help clear up the logic
                    .unwrap();

                ExecuteOptimisticBlockStreamRequest {
                    base_block: Some(base_block),
                }
            });

            async move {
                let stream = client.execute_optimistic_block_stream(requests).await?;
                Ok((stream, opts_to_exec_tx))
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
