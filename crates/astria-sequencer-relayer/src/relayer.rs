use std::str::FromStr;

use eyre::{
    Result,
    WrapErr as _,
};
use tendermint::account::Id as AccountId;
use tokio::{
    sync::{
        mpsc::UnboundedSender,
        watch,
    },
    task::JoinHandle,
    time::Interval,
};
use tracing::{
    info,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
    validator::Validator,
};

pub struct Relayer {
    sequencer_client: SequencerClient,
    da_client: CelestiaClient,
    disable_writing: bool,
    validator: Validator,
    interval: Interval,
    block_tx: UnboundedSender<SequencerBlock>,

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
        da_client: CelestiaClient,
        interval: Interval,
        block_tx: UnboundedSender<SequencerBlock>,
    ) -> Result<Self> {
        let validator = Validator::from_path(cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let (state, _) = watch::channel(State::default());

        Ok(Self {
            sequencer_client,
            da_client,
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

        info!(height = ?height, tx_count = resp.block.data.txs.len(), "got block from sequencer");
        new_state.current_sequencer_height.replace(height);

        if resp.block.header.proposer_address.as_ref() != self.validator.address.as_ref() {
            let proposer_address =
                AccountId::try_from(resp.block.header.proposer_address.0.clone())
                    .wrap_err("failed to convert proposer address")?;

            info!(
                %proposer_address,
                validator_address = %self.validator.address,
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
        if self.disable_writing {
            return Ok(new_state);
        }

        let tx_count = sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
        match self
            .da_client
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
