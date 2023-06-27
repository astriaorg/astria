use std::sync::Arc;

use ed25519_consensus::SigningKey;
use eyre::WrapErr as _;
use tokio::{
    sync::watch,
    time::Interval,
};
use tracing::{
    info,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    sequencer_block::SequencerBlock,
    validator::Validator,
};

pub struct Writer {
    validator: Validator,
    da_client: Arc<CelestiaClient>,
    da_block_interval: Interval,
    block_rx: watch::Receiver<Option<SequencerBlock>>,
}

impl Writer {
    pub fn new(
        cfg: crate::config::Config,
        da_client: CelestiaClient,
        da_block_interval: Interval,
        block_rx: watch::Receiver<Option<SequencerBlock>>,
    ) -> eyre::Result<Self> {
        let validator = Validator::from_path(cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        Ok(Self {
            validator,
            da_client: Arc::new(da_client),
            da_block_interval,
            block_rx,
        })
    }

    pub fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = self.da_block_interval.tick() => {
                        self.handle_celestia_block_tick();
                    }
                    res = self.block_rx.changed() => {
                        if let Err(e) = res {
                            warn!(error = ?e, "block_rx channel closed");
                            break;
                        };

                        let Some(sequencer_block) = self.block_rx.borrow().clone() else {
                            panic!("block_rx should not receive None")
                        };

                        match self.handle_new_block(sequencer_block) {
                            Ok(_) => {}
                            Err(e) => warn!(error = ?e, "failed to handle new block"),
                        }
                    }
                }
            }
        })
    }

    fn handle_new_block(&self, sequencer_block: SequencerBlock) -> eyre::Result<()> {
        let tx_count = sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
        let height = sequencer_block.header.height.clone();
        let da_client = self.da_client.clone();
        let signing_key_bytes = self.validator.signing_key.to_bytes();
        let signing_key = SigningKey::from(signing_key_bytes);

        // TODO: batch this!!!!!
        tokio::task::spawn(async move {
            match da_client
                .submit_block(
                    sequencer_block,
                    &signing_key,
                    signing_key.verification_key(),
                )
                .await
            {
                Ok(resp) => {
                    // new_state
                    //     .current_data_availability_height
                    //     .replace(resp.height);
                    info!(
                        sequencer_block = height,
                        da_layer_block = resp.height,
                        tx_count,
                        "submitted sequencer block to DA layer",
                    );
                }
                Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
            }
        });

        // TODO: deal with updating DA height
        // Ok(new_state)

        Ok(())
    }

    fn handle_celestia_block_tick(&self) {}
}
