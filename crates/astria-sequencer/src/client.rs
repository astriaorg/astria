use anyhow::Context;
use astria_core::generated::{
    composer::v1alpha1::{
        sequencer_hooks_service_client::SequencerHooksServiceClient,
        SendFinalizedHashRequest,
        SendFinalizedHashResponse,
        SendOptimisticBlockRequest,
        SendOptimisticBlockResponse,
    },
    protocol::transactions::v1alpha1::SequenceAction,
};
use bytes::Bytes;
use tendermint::Time;
use tendermint_proto::google::protobuf::Timestamp;
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tracing::{
    info,
    instrument,
};

/// A newtype wrapper around [`SequencerHooksServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(crate) struct SequencerHooksClient {
    uri: Uri,
    enabled: bool,
    inner: SequencerHooksServiceClient<Channel>,
}

impl SequencerHooksClient {
    pub(crate) fn connect_lazy(uri: &str, enabled: bool) -> anyhow::Result<Self> {
        let uri: Uri = uri
            .parse()
            .context("failed to parse provided string as uri")?;
        let endpoint = Endpoint::from(uri.clone()).connect_lazy();
        let inner = SequencerHooksServiceClient::new(endpoint);
        Ok(Self {
            uri,
            enabled,
            inner,
        })
    }

    // pub(crate) fn uri(&self) -> String {
    //     self.uri.to_string()
    // }

    #[instrument(skip_all, fields(uri = % self.uri), err)]
    pub(super) async fn send_optimistic_block(
        &self,
        block_hash: Bytes,
        seq_actions: Vec<SequenceAction>,
        time: Time,
    ) -> anyhow::Result<SendOptimisticBlockResponse> {
        if !self.enabled {
            info!("BHARATH: optimistic block sending is disabled");
            return Ok(SendOptimisticBlockResponse::default());
        }
        info!(
            "BHARATH: sending optimistic block hash to {:?}",
            self.uri.to_string()
        );

        let Timestamp {
            seconds,
            nanos,
        } = time.into();

        info!("BHARATH: seconds: {:?}, nanos: {:?}", seconds, nanos);

        let request = SendOptimisticBlockRequest {
            block_hash,
            seq_action: seq_actions,
            time: Some(pbjson_types::Timestamp {
                seconds,
                nanos,
            }),
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
        if !self.enabled {
            info!("BHARATH: finalized block hash sending is disabled");
            return Ok(SendFinalizedHashResponse::default());
        }
        info!(
            "BHARATH: sending finalized block hash to {:?}",
            self.uri.to_string()
        );
        let request = SendFinalizedHashRequest {
            block_hash: finalized_block_hash,
        };

        let mut client = self.inner.clone();
        let response = client.send_finalized_hash(request).await?;

        Ok(response.into_inner())
    }
}
