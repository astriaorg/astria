use std::{
    sync::Arc,
    time::Duration,
};

use astria_eyre::eyre::{
    self,
    ensure,
    Context as _,
};
use sequencer_client::tendermint_rpc;
use tokio_util::sync::CancellationToken;
use tracing::warn;

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
    pub(crate) async fn build(self) -> eyre::Result<(super::Submitter, super::Handle)> {
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

        let actual_chain_id = get_sequencer_chain_id(sequencer_cometbft_client.clone()).await?;
        ensure!(
            sequencer_chain_id == actual_chain_id.to_string(),
            "sequencer_chain_id provided in config does not match chain_id returned from sequencer"
        );

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

async fn get_sequencer_chain_id(
    client: sequencer_client::HttpClient,
) -> eyre::Result<tendermint::chain::Id> {
    use sequencer_client::Client as _;

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch sequencer genesis info; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let genesis: tendermint::Genesis = tryhard::retry_fn(|| client.genesis())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get genesis info from Sequencer after a lot of attempts")?;

    Ok(genesis.chain_id)
}
