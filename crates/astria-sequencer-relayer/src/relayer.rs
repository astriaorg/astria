use std::{
    str::FromStr,
    time::Duration,
};

use bech32::{
    self,
    ToBase32,
    Variant,
};
use eyre::{
    Result,
    WrapErr as _,
};
use tokio::{
    sync::{
        mpsc::UnboundedSender,
        watch::{
            self,
            Receiver,
        },
    },
    time,
};
use tracing::{
    debug,
    info,
    warn,
};

use crate::{
    data_availability::{
        CelestiaClient,
        CelestiaClientBuilder,
    },
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
    validator::Validator,
};

pub struct Relayer {
    sequencer_client: SequencerClient,
    data_availability_client: Option<CelestiaClient>,
    validator: Validator,
    sequencer_poll_period: Duration,
    block_tx: UnboundedSender<SequencerBlock>,
    state_tx: watch::Sender<State>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub(crate) current_sequencer_height: Option<u64>,
    pub(crate) current_data_availability_height: Option<u64>,
}

impl State {
    pub fn is_ready(&self) -> bool {
        self.current_sequencer_height.is_some() && self.current_data_availability_height.is_some()
    }
}

impl Relayer {
    /// Instantiates a `Relayer`.
    ///
    /// # Errors
    ///
    /// Returns one of the following errors:
    /// + failed to read the validator keys from the path in cfg;
    /// + failed to construct a client to the sequencer;
    /// + failed to construct a client to the data availability layer (if cfg.disable_writing is
    /// set)
    pub fn new(
        cfg: &crate::config::Config,
        block_tx: UnboundedSender<SequencerBlock>,
    ) -> Result<Self> {
        let validator = Validator::from_path(&cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let sequencer_client = SequencerClient::new(cfg.sequencer_endpoint.clone())
            .wrap_err("failed to create sequencer client")?;

        let data_availability_client = if cfg.disable_writing {
            debug!("disabling writing to data availability layer requested; disabling");
            None
        } else {
            let client = CelestiaClientBuilder::new(cfg.celestia_endpoint.clone())
                .gas_limit(cfg.gas_limit)
                .build()
                .wrap_err("failed to create data availability client")?;
            Some(client)
        };

        let (state_tx, _) = watch::channel(State::default());

        Ok(Self {
            sequencer_client,
            data_availability_client,
            sequencer_poll_period: Duration::from_millis(cfg.block_time),
            validator,
            block_tx,
            state_tx,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> Receiver<State> {
        self.state_tx.subscribe()
    }

    async fn get_and_submit_latest_block(&self) -> eyre::Result<State> {
        let mut new_state = (*self.state_tx.borrow()).clone();
        let resp = self.sequencer_client.get_latest_block().await?;

        let maybe_height: Result<u64, <u64 as FromStr>::Err> = resp.block.header.height.parse();
        if let Err(e) = maybe_height {
            warn!(
                error = ?e,
                "got invalid block height {} from sequencer",
                resp.block.header.height,
            );
            return Ok(new_state);
        }

        let height = maybe_height.unwrap();
        if height <= *new_state.current_sequencer_height.get_or_insert(height) {
            return Ok(new_state);
        }

        info!("got block with height {} from sequencer", height);
        new_state.current_sequencer_height.replace(height);

        if resp.block.header.proposer_address.as_ref() != self.validator.address.as_ref() {
            let proposer_address = bech32::encode(
                "metrovalcons",
                resp.block.header.proposer_address.0.to_base32(),
                Variant::Bech32,
            )
            .expect("should encode block proposer address");
            info!(
                %proposer_address,
                validator_address = %self.validator.bech32_address,
                "ignoring block: proposer address is not ours",
            );
            return Ok(new_state);
        }

        let sequencer_block = match SequencerBlock::from_cosmos_block(resp.block) {
            Ok(block) => block,
            Err(e) => {
                warn!(error = ?e, "failed to convert block to DA block");
                return Ok(new_state);
            }
        };

        self.block_tx.send(sequencer_block.clone())?;
        let tx_count = sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
        if let Some(client) = &self.data_availability_client {
            match client
                .submit_block(
                    sequencer_block,
                    &self.validator.signing_key,
                    self.validator.verification_key,
                )
                .await
            {
                Ok(resp) => {
                    new_state
                        .current_data_availability_height
                        .replace(resp.height);
                    info!(
                        sequencer_block = height,
                        da_layer_block = resp.height,
                        tx_count,
                        "submitted sequencer block to DA layer",
                    );
                }
                Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
            }
        }
        Ok(new_state)
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// `Relayer::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut interval = time::interval(self.sequencer_poll_period);
        loop {
            interval.tick().await;
            match self.get_and_submit_latest_block().await {
                Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
                Ok(new_state) if new_state != *self.state_tx.borrow() => {
                    _ = self.state_tx.send_replace(new_state);
                }
                Ok(_) => {}
            }
        }
        // Return Ok to make the types align (see the method's doc comment why this is necessary).
        // Allow unreachable code to quiet warnings
        #[allow(unreachable_code)]
        Ok(())
    }
}
