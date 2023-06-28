use std::str::FromStr;

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
    sync::watch,
    task::JoinHandle,
    time::Interval,
};
use tracing::{
    info,
    warn,
};

use crate::{
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
    validator::Validator,
};

pub struct Relayer {
    sequencer_client: SequencerClient,
    disable_writing: bool,
    validator: Validator,
    interval: Interval,
    block_tx: watch::Sender<Option<SequencerBlock>>,

    state: watch::Sender<State>,
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
    pub fn new(
        cfg: crate::config::Config,
        sequencer_client: SequencerClient,
        interval: Interval,
        block_tx: watch::Sender<Option<SequencerBlock>>,
    ) -> Result<Self> {
        let validator = Validator::from_path(cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let (state, _) = watch::channel(State::default());

        Ok(Self {
            sequencer_client,
            disable_writing: false,
            validator,
            interval,
            block_tx,
            state,
        })
    }

    pub fn disable_writing(&mut self) {
        self.disable_writing = true;
    }

    pub fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state.subscribe()
    }

    async fn get_and_submit_latest_block(&self) -> eyre::Result<State> {
        let mut new_state = (*self.state.borrow()).clone();
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

        self.block_tx.send(Some(sequencer_block))?;
        if self.disable_writing {
            return Ok(new_state);
        }

        Ok(new_state)
    }

    pub fn run(mut self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                self.interval.tick().await;
                match self.get_and_submit_latest_block().await {
                    Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
                    Ok(new_state) if new_state != *self.state.borrow() => {
                        _ = self.state.send_replace(new_state);
                    }
                    Ok(_) => {}
                }
            }
        })
    }
}
