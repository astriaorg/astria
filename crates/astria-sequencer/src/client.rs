use anyhow::Context;
use astria_core::generated::{
    composer::v1alpha1::{
        sequencer_hooks_service_client::SequencerHooksServiceClient,
        SendFinalizedHashRequest,
        SendFinalizedHashResponse,
        SendOptimisticBlockRequest,
        SendOptimisticBlockResponse,
    },
    protocol::transaction::v1alpha1::SequenceAction,
};
use bytes::Bytes;
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    instrument,
    Instrument,
};

/// A newtype wrapper around [`SequencerHooksServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(crate) struct SequencerHooksClient {
    uri: Uri,
    inner: SequencerHooksServiceClient<Channel>,
}

impl SequencerHooksClient {
    pub(crate) fn connect_lazy(uri: &str) -> anyhow::Result<Self> {
        let uri: Uri = uri
            .parse()
            .context("failed to parse provided string as uri")?;
        let endpoint = Endpoint::from(uri.clone()).connect_lazy();
        let inner = SequencerHooksServiceClient::new(endpoint);
        Ok(Self {
            uri,
            inner,
        })
    }

    pub(crate) fn uri(&self) -> String {
        self.uri.to_string()
    }

    #[instrument(skip_all, fields(uri = % self.uri), err)]
    pub(super) async fn send_optimistic_block(
        &self,
        block_hash: Bytes,
        seq_actions: Vec<SequenceAction>,
    ) -> anyhow::Result<SendOptimisticBlockResponse> {
        let request = SendOptimisticBlockRequest {
            block_hash,
            seq_action: seq_actions,
        };

        let mut client = self.inner.clone();
        let response = client.send_optimistic_block(request).await?;

        Ok(response.into_inner())
    }

    #[instrument(skip_all, fields(uri = % self.uri), err)]
    pub(super) async fn send_finalized_block_hash(
        &self,
        finalized_block_hash: Bytes,
    ) -> anyhow::Result<SendFinalizedHashResponse> {
        let request = SendFinalizedHashRequest {
            block_hash: finalized_block_hash,
        };

        let mut client = self.inner.clone();
        let response = client.send_finalized_hash(request).await?;

        Ok(response.into_inner())
    }
}
