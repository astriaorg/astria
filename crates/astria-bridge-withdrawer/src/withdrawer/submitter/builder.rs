use std::sync::Arc;

use astria_eyre::eyre::{
    self,
    Context as _,
};
use tokio_util::sync::CancellationToken;

use super::state::State;

const BATCH_QUEUE_SIZE: usize = 256;

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) sequencer_key_path: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) cometbft_endpoint: String,
    pub(crate) state: Arc<State>,
}

impl Builder {
    /// Instantiates an `Submitter`.
    pub(crate) fn build(self) -> eyre::Result<(super::Submitter, super::Handle)> {
        let Self {
            shutdown_token,
            sequencer_key_path,
            sequencer_chain_id,
            cometbft_endpoint,
            state,
        } = self;

        let signer = super::signer::SequencerSigner::from_path(sequencer_key_path)?;
        let (batches_tx, batches_rx) = tokio::sync::mpsc::channel(BATCH_QUEUE_SIZE);

        let sequencer_cometbft_client = sequencer_client::HttpClient::new(&*cometbft_endpoint)
            .context("failed constructing cometbft http client")?;

        Ok((
            super::Submitter {
                shutdown_token,
                state,
                batches_rx,
                signer,
                sequencer_chain_id,
                sequencer_cometbft_client,
            },
            super::Handle {
                batches_tx,
            },
        ))
    }
}
