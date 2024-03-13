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
use tracing::instrument;

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
        let req = GetFilteredSequencerBlockRequest {
            height,
            rollup_ids: vec![rollup_id.to_vec()],
        };
        let raw_block = self
            .inner
            .get_filtered_sequencer_block(req)
            .await
            .wrap_err("failed fetching sequencer block information for rollup")?
            .into_inner();
        FilteredSequencerBlock::try_from_raw(raw_block)
            .wrap_err("failed validating filtered block response")
    }
}
