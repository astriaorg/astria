use std::sync::Arc;

use ed25519_dalek::Keypair;
use tokio::{
    sync::watch,
    time::Interval,
};
use tracing::{
    info,
    warn,
};

use crate::{
    base64_string::Base64String,
    data_availability::CelestiaClient,
    keys::private_key_bytes_to_keypair,
    relayer::ValidatorPrivateKeyFile,
    sequencer_block::SequencerBlock,
};

pub struct Writer {
    keypair: ed25519_dalek::Keypair,
    da_client: Arc<CelestiaClient>,
    da_block_interval: Interval,
    block_rx: watch::Receiver<Option<SequencerBlock>>,
}

impl Writer {
    pub fn new(
        key_file: ValidatorPrivateKeyFile,
        da_client: CelestiaClient,
        da_block_interval: Interval,
        block_rx: watch::Receiver<Option<SequencerBlock>>,
    ) -> Self {
        // generate our private-public keypair
        let keypair = private_key_bytes_to_keypair(
            &Base64String::from_string(key_file.priv_key.value)
                .expect("failed to decode validator private key; must be base64 string")
                .0,
        )
        .expect("failed to convert validator private key to keypair");

        Self {
            keypair,
            da_client: Arc::new(da_client),
            da_block_interval,
            block_rx,
        }
    }

    pub fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                let res = self.block_rx.changed().await;
                if let Err(e) = res {
                    warn!(error = ?e, "block_rx channel closed");
                    break;
                };

                let Some(sequencer_block) = self.block_rx.borrow().clone() else {
                    panic!("block_rx should not receive None")
                };

                let tx_count =
                    sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
                let height = sequencer_block.header.height.clone();
                let da_client = self.da_client.clone();
                let keypair_bytes = self.keypair.to_bytes();
                let keypair = Keypair::from_bytes(&keypair_bytes).expect("should copy keypair");

                tokio::task::spawn(async move {
                    match da_client.submit_block(sequencer_block, &keypair).await {
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
            }
        })
    }
}
