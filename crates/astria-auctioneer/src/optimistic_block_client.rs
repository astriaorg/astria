use astria_core::{
    generated::sequencerblock::optimisticblock::v1alpha1::{
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
use tracing::instrument;

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

    #[instrument(skip_all, fields(
        uri = %self.uri,
        %rollup_id,
        err,
    ))]
    pub(super) async fn get_optimistic_block_stream(
        &mut self,
        rollup_id: RollupId,
    ) -> eyre::Result<tonic::Streaming<GetOptimisticBlockStreamResponse>> {
        let stream = self
            .inner
            .get_optimistic_block_stream(GetOptimisticBlockStreamRequest {
                rollup_id: Some(rollup_id.into_raw()),
            })
            .await
            .wrap_err("failed to open optimistic block stream")?
            .into_inner();
        Ok(stream)
    }

    #[instrument(skip_all, fields(
           uri = %self.uri,
           err,
       ))]
    pub(super) async fn get_block_commitment_stream(
        &mut self,
    ) -> eyre::Result<tonic::Streaming<GetBlockCommitmentStreamResponse>> {
        let stream = self
            .inner
            .get_block_commitment_stream(GetBlockCommitmentStreamRequest {})
            .await
            .wrap_err("failed to open block commitment stream")?
            .into_inner();
        Ok(stream)
    }
}
