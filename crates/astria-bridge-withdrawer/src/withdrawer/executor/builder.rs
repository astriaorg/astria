use std::sync::Arc;

use astria_eyre::eyre::{
    self,
    Context as _,
};
use tokio_util::sync::CancellationToken;

use super::state::State;

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) sequencer_key_path: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) cometbft_endpoint: String,
}

impl Builder {
    /// Instantiates an `Executor`.
    pub(crate) fn build(self) -> eyre::Result<(super::Executor, super::Handle)> {
        let Self {
            shutdown_token,
            sequencer_key_path,
            sequencer_chain_id,
            cometbft_endpoint,
        } = self;

        let signer = super::signer::SequencerSigner::from_path(sequencer_key_path)?;
        let (batches_tx, batches_rx) = tokio::sync::mpsc::channel(1);

        let sequencer_cometbft_client = sequencer_client::HttpClient::new(&*cometbft_endpoint)
            .context("failed constructing cometbft http client")?;

        let state = Arc::new(State::new());

        Ok((
            super::Executor {
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
