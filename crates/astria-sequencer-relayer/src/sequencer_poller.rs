use std::time::Duration;

use astria_sequencer_client::{
    BlockResponse,
    Client,
};
use eyre::WrapErr as _;
use tokio::{
    sync::mpsc::UnboundedSender,
    time::interval,
};
use tracing::warn;

use crate::config::Config;
pub(crate) struct SequencerPoller {
    client: Client,
    poll_period: Duration,
    sequencer_blocks_tx: UnboundedSender<BlockResponse>,
}

impl SequencerPoller {
    pub(crate) fn new(
        cfg: &Config,
        sequencer_blocks_tx: UnboundedSender<BlockResponse>,
    ) -> eyre::Result<Self> {
        let client =
            Client::new(&cfg.sequencer_endpoint).wrap_err("failed to create sequencer client")?;
        Ok(Self {
            client,
            poll_period: Duration::from_millis(cfg.block_time),
            sequencer_blocks_tx,
        })
    }

    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut interval = interval(self.poll_period);
        loop {
            interval.tick().await;
            match self.client.get_latest_block().await {
                Ok(block) => self
                    .sequencer_blocks_tx
                    .send(block)
                    .wrap_err("channel closed unexpectedtly")?,
                Err(e) => {
                    warn!(error.msg = %e, error.cause_chain = ?e, "failed getting latest block from sequencer")
                }
            }
        }
        // Return Ok to make the types align (see the method's doc comment why this is necessary).
        // Allow unreachable code to quiet warnings
        #[allow(unreachable_code)]
        Ok(())
    }
}
