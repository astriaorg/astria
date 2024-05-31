use std::sync::Arc;

use astria_eyre::eyre::{
    self,
    Context as _,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::state::State;
use crate::withdrawer::submitter::Batch;

const BATCH_QUEUE_SIZE: usize = 256;

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) sequencer_key_path: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) sequencer_cometbft_endpoint: String,
    pub(crate) state: Arc<State>,
}

impl Builder {
    /// Instantiates an `Submitter`.
    pub(crate) fn build(self) -> eyre::Result<(super::Submitter, mpsc::Sender<Batch>)> {
        let Self {
            shutdown_token,
            sequencer_key_path,
            sequencer_chain_id,
            sequencer_cometbft_endpoint,
            state,
        } = self;

        let signer = super::signer::SequencerKey::try_from_path(sequencer_key_path)
            .wrap_err("failed to load sequencer private ky")?;
        info!(address = %telemetry::display::hex(&signer.address), "loaded sequencer signer");
        let (batches_tx, batches_rx) = tokio::sync::mpsc::channel(BATCH_QUEUE_SIZE);

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_cometbft_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        Ok((
            super::Submitter {
                shutdown_token,
                state,
                batches_rx,
                sequencer_cometbft_client,
                signer,
                sequencer_chain_id,
            },
            batches_tx,
        ))
    }
}
