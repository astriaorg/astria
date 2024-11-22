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
use tracing::instrument;

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
        let (blocks_to_execute_tx, requests) = make_execution_requests_stream(rollup_id);
        let stream = self
            .inner
            .execute_optimistic_block_stream(requests)
            .await
            .wrap_err("failed to open execute optimistic block stream")?
            .into_inner();
        Ok((stream, blocks_to_execute_tx))
    }
}
