use std::sync::Arc;

use astria_core::generated::astria::sequencerblock::v1::sequencer_service_client::SequencerServiceClient;
use astria_eyre::eyre::{
    self,
    Context as _,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    instrument,
};

use super::state::State;
use crate::{
    bridge_withdrawer::{
        startup,
        submitter::Batch,
    },
    metrics::Metrics,
};

const BATCH_QUEUE_SIZE: usize = 256;

pub(crate) struct Handle {
    batches_tx: mpsc::Sender<Batch>,
}

impl Handle {
    pub(crate) fn new(batches_tx: mpsc::Sender<Batch>) -> Self {
        Self {
            batches_tx,
        }
    }

    #[instrument(skip_all, err)]
    pub(crate) async fn send_batch(&self, batch: Batch) -> eyre::Result<()> {
        self.batches_tx
            .send(batch)
            .await
            .wrap_err("failed send batch")
    }
}

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) startup_handle: startup::InfoHandle,
    pub(crate) sequencer_key_path: String,
    pub(crate) sequencer_address_prefix: String,
    pub(crate) sequencer_cometbft_client: sequencer_client::HttpClient,
    pub(crate) sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
    pub(crate) state: Arc<State>,
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    /// Instantiates an `Submitter`.
    pub(crate) fn build(self) -> eyre::Result<(super::Submitter, Handle)> {
        let Self {
            shutdown_token,
            startup_handle,
            sequencer_key_path,
            sequencer_address_prefix,
            sequencer_cometbft_client,
            sequencer_grpc_client,
            state,
            metrics,
        } = self;

        let signer = super::signer::SequencerKey::builder()
            .path(sequencer_key_path)
            .prefix(sequencer_address_prefix)
            .try_build()
            .wrap_err("failed to load sequencer private key")?;
        info!(address = %signer.address(), "loaded sequencer signer");

        let (batches_tx, batches_rx) = tokio::sync::mpsc::channel(BATCH_QUEUE_SIZE);
        let handle = Handle::new(batches_tx);

        Ok((
            super::Submitter {
                shutdown_token,
                startup_handle,
                state,
                batches_rx,
                sequencer_cometbft_client,
                sequencer_grpc_client,
                signer,
                metrics,
            },
            handle,
        ))
    }
}
