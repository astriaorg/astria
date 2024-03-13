use std::time::Duration;

use astria_core::{
    generated::sequencer::v1alpha1::{
        sequencer_service_client::SequencerServiceClient,
        GetFilteredSequencerBlockRequest,
    },
    sequencer::v1alpha1::{
        block::FilteredSequencerBlock,
        RollupId,
    },
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
    Instrument,
};

#[derive(Clone)]
pub(crate) struct SequencerGrpcClient {
    inner: SequencerServiceClient<Channel>,
    uri: Uri,
}

impl SequencerGrpcClient {
    pub(crate) fn new(sequencer_uri: &str) -> eyre::Result<Self> {
        let uri: Uri = sequencer_uri
            .parse()
            .wrap_err("failed parsing provided string as Uri")?;
        let endpoint = Endpoint::from(uri.clone());
        let inner = SequencerServiceClient::new(endpoint.connect_lazy());
        Ok(Self {
            inner,
            uri,
        })
    }

    #[instrument(skip_all, fields(
        uri = %self.uri,
        height,
        %rollup_id,
        err,
    ))]
    pub(super) async fn get(
        &mut self,
        height: u64,
        rollup_id: RollupId,
    ) -> eyre::Result<FilteredSequencerBlock> {
        let span = tracing::Span::current();
        let retry_cfg = tryhard::RetryFutureConfig::new(u32::MAX)
            .exponential_backoff(Duration::from_millis(100))
            .max_delay(Duration::from_secs(5))
            .on_retry(
                |attempt: u32, next_delay: Option<Duration>, error: &tonic::Status| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        parent: &span,
                        attempt,
                        wait_duration,
                        error = error as &dyn std::error::Error,
                        "attempt to grab sequencer block failed; retrying after backoff",
                    );
                    futures::future::ready(())
                },
            );

        let client = self.inner.clone();
        let raw_block = tryhard::retry_fn(|| {
            let mut client = client.clone();
            let req = GetFilteredSequencerBlockRequest {
                height,
                rollup_ids: vec![rollup_id.to_vec()],
            };
            async move { client.get_filtered_sequencer_block(req).await }
        })
        .with_config(retry_cfg)
        .in_current_span()
        .await
        .wrap_err("failed fetching filtered block after a lot of retries, bailing")?
        .into_inner();
        FilteredSequencerBlock::try_from_raw(raw_block)
            .wrap_err("failed validating filtered block response")
    }
}
