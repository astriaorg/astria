use std::sync::Arc;

use astria_core::primitive::v1::asset;
use astria_eyre::eyre::{
    self,
    Context as _,
};
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::state::State;
use crate::{
    bridge_withdrawer::{
        submitter::Batch,
        SequencerStartupInfo,
    },
    metrics::Metrics,
};

const BATCH_QUEUE_SIZE: usize = 256;

pub(crate) struct Handle {
    startup_info_rx: Option<oneshot::Receiver<SequencerStartupInfo>>,
    batches_tx: mpsc::Sender<Batch>,
}

impl Handle {
    pub(crate) fn new(
        startup_info_rx: oneshot::Receiver<SequencerStartupInfo>,
        batches_tx: mpsc::Sender<Batch>,
    ) -> Self {
        Self {
            startup_info_rx: Some(startup_info_rx),
            batches_tx,
        }
    }

    pub(crate) async fn recv_startup_info(&mut self) -> eyre::Result<SequencerStartupInfo> {
        self.startup_info_rx
            .take()
            .expect("startup info should only be taken once - this is a bug")
            .await
            .wrap_err("failed to get startup info from submitter. channel was dropped.")
    }

    pub(crate) async fn send_batch(&self, batch: Batch) -> eyre::Result<()> {
        self.batches_tx
            .send(batch)
            .await
            .wrap_err("failed to send batch")
    }
}

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) sequencer_key_path: String,
    pub(crate) sequencer_address_prefix: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) sequencer_cometbft_endpoint: String,
    pub(crate) state: Arc<State>,
    pub(crate) expected_fee_asset_id: asset::Id,
    pub(crate) min_expected_fee_asset_balance: u128,
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    /// Instantiates an `Submitter`.
    pub(crate) fn build(self) -> eyre::Result<(super::Submitter, Handle)> {
        let Self {
            shutdown_token,
            sequencer_key_path,
            sequencer_address_prefix,
            sequencer_chain_id,
            sequencer_cometbft_endpoint,
            state,
            expected_fee_asset_id,
            min_expected_fee_asset_balance,
            metrics,
        } = self;

        let signer = super::signer::SequencerKey::builder()
            .path(sequencer_key_path)
            .prefix(sequencer_address_prefix)
            .try_build()
            .wrap_err("failed to load sequencer private key")?;
        info!(address = %signer.address(), "loaded sequencer signer");

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_cometbft_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        let (batches_tx, batches_rx) = tokio::sync::mpsc::channel(BATCH_QUEUE_SIZE);
        let (startup_tx, startup_rx) = tokio::sync::oneshot::channel();
        let handle = Handle::new(startup_rx, batches_tx);

        Ok((
            super::Submitter {
                shutdown_token,
                state,
                batches_rx,
                sequencer_cometbft_client,
                signer,
                sequencer_chain_id,
                startup_tx,
                expected_fee_asset_id,
                min_expected_fee_asset_balance,
                metrics,
            },
            handle,
        ))
    }
}
