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
use tracing::debug;

use crate::{
    config::Config,
    macros::report_err,
};

/// `SequencerPoller` polls the sequencer for new blocks at fixed intervals and sends them to the
/// relayer for further processing.
pub(crate) struct SequencerPoller {
    /// The actual client used to poll the sequencer.
    client: Client,

    /// The poll period defines the fixed interval at which the sequencer is polled.
    poll_period: Duration,

    /// The channel over which new responses are sent to the relayer for further processing.
    relayer_tx: UnboundedSender<BlockResponse>,

    /// The recently seen block hash recorded in the block response. This is used to
    /// determine if a block is actually new. Note that this rests on the assumption
    /// that the sequencer always sends the current or a newer block, but never an
    /// older one.
    previous_block_hash: Option<tendermint::Hash>,
}

impl SequencerPoller {
    /// Create a new sequencer poller from the given config and a channel to the relayer.
    pub(crate) fn new(
        cfg: &Config,
        relayer_tx: UnboundedSender<BlockResponse>,
    ) -> eyre::Result<Self> {
        let client =
            Client::new(&cfg.sequencer_endpoint).wrap_err("failed to create sequencer client")?;
        Ok(Self {
            client,
            poll_period: Duration::from_millis(cfg.block_time),
            relayer_tx,
            previous_block_hash: None,
        })
    }

    /// Poll the sequencer indefinitely.
    //
    /// # Errors
    ///
    /// Returns an error if the channel to send new blocks to the relayer closed.
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        use base64::{
            engine::general_purpose::STANDARD,
            Engine as _,
        };
        let mut interval = interval(self.poll_period);
        loop {
            interval.tick().await;
            match self.client.get_latest_block().await {
                // Drop the block if we have just seen it.
                Ok(block) if Some(block.block_id.hash) == self.previous_block_hash => {
                    debug!(
                        block.hash = STANDARD.encode(&block.block_id.hash),
                        "block previously received; dropping"
                    );
                }
                Ok(block) => {
                    self.previous_block_hash.replace(block.block_id.hash);
                    self.relayer_tx
                        .send(block)
                        .wrap_err("channel to send new blocks to the relayer closed unexpectedly")?
                }

                Err(e) => report_err!(e, "failed getting latest block from sequencer"),
            }
        }
        // Return Ok to make the types align.
        //
        // Currently there is no break point in the loop, so we need to allow this
        // to quiet compiler warnings.
        #[allow(unreachable_code)]
        Ok(())
    }
}
