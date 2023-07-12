use astria_sequencer_client::BlockResponse;
use eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use tokio::sync::{
    mpsc::{
        UnboundedReceiver,
        UnboundedSender,
    },
    watch::{
        self,
        Receiver,
    },
};
use tracing::{
    debug,
    instrument,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    types::SequencerBlockData,
    validator::Validator,
};

pub struct Relayer {
    data_availability_client: Option<CelestiaClient>,
    validator: Validator,
    sequencer_blocks_rx: UnboundedReceiver<BlockResponse>,
    block_tx: UnboundedSender<SequencerBlockData>,
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
    /// + failed to construct a client to the data availability layer (unless `cfg.disable_writing`
    ///   is set).
    pub fn new(
        cfg: &crate::config::Config,
        sequencer_blocks_rx: UnboundedReceiver<BlockResponse>,
        block_tx: UnboundedSender<SequencerBlockData>,
    ) -> Result<Self> {
        let validator = Validator::from_path(&cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let data_availability_client = if cfg.disable_writing {
            debug!("disabling writing to data availability layer requested; disabling");
            None
        } else {
            let client = CelestiaClient::builder()
                .endpoint(&cfg.celestia_endpoint)
                .bearer_token(&cfg.celestia_bearer_token)
                .gas_limit(cfg.gas_limit)
                .build()
                .wrap_err("failed to create data availability client")?;
            Some(client)
        };

        let (state_tx, _) = watch::channel(State::default());

        Ok(Self {
            data_availability_client,
            validator,
            sequencer_blocks_rx,
            block_tx,
            state_tx,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> Receiver<State> {
        self.state_tx.subscribe()
    }

    fn convert_block_response_to_sequencer_block_data(
        &self,
        res: BlockResponse,
        current_height: Option<u64>,
    ) -> eyre::Result<SequencerBlockData> {
        if let Some(current_height) = current_height {
            ensure!(
                res.block.header.height.value() > current_height,
                "sequencer block response had height below current height tracked in relayer"
            );
        }
        ensure!(
            res.block.header.proposer_address == self.validator.address,
            "proposer recorded in sequencer block does not match internal validator"
        );
        let sequencer_block_data = SequencerBlockData::from_tendermint_block(res.block)
            .wrap_err("failed converting sequencer block response to sequencer block data")?;
        Ok(sequencer_block_data)
    }

    #[instrument(skip_all)]
    async fn submit_blocks(&self, block_responses: Vec<BlockResponse>) -> State {
        let mut new_state = (*self.state_tx.borrow()).clone();
        let mut all_converted_blocks = Vec::with_capacity(block_responses.len());
        for res in block_responses {
            match self.convert_block_response_to_sequencer_block_data(
                res,
                new_state.current_sequencer_height,
            ) {
                Ok(converted) => {
                    if let Err(error) = self.block_tx.send(converted.clone()) {
                        warn!(?error, "failed sending sequencer block data to gossip task");
                    }
                    all_converted_blocks.push(converted)
                }
                Err(error) => {
                    // TODO: better event field, maybe with a way to identify the block?
                    warn!(?error, "dropping block response");
                }
            }
        }
        // get the max sequencer height from the valid blocks; the result of this op is `None` if
        // the vector is empty.
        let Some(max_sequencer_height) = all_converted_blocks
            .iter()
            .map(|block| block.header.height.value())
            .max()
        else {
            warn!(
                "no blocks remained after conversion; not submitting blocks to data availability \
                 layer"
            );
            return new_state;
        };
        new_state
            .current_sequencer_height
            .replace(max_sequencer_height);
        if let Some(client) = &self.data_availability_client {
            match client
                .submit_all_blocks(
                    all_converted_blocks,
                    &self.validator.signing_key,
                    self.validator.verification_key,
                )
                .await
            {
                Ok(res) => {
                    new_state
                        .current_data_availability_height
                        .replace(res.height);
                }
                Err(e) => warn!(
                    error.msg = %e,
                    error.cause_chain = ?e,
                    "failed to submit block to data availability layer",
                ),
            }
        }
        new_state
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// `Relayer::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        use tokio::sync::mpsc::error::TryRecvError;
        loop {
            // First wait until a new block is available
            let Some(new_block) = self.sequencer_blocks_rx.recv().await else {
                bail!("sequencer block channel closed unexpectedly");
            };
            let mut new_blocks = vec![new_block];
            // Then drain the channel
            'drain_channel: loop {
                match self.sequencer_blocks_rx.try_recv() {
                    Ok(block) => new_blocks.push(block),
                    Err(e) => {
                        if matches!(e, TryRecvError::Disconnected) {
                            warn!(
                                num_outstanding = new_blocks.len(),
                                "sequencer sequencer block channel closed unexpectedly; \
                                 attempting to submit outstanding blocks to data availability \
                                 layer"
                            );
                        }
                        break 'drain_channel;
                    }
                }
            }
            let new_state = self.submit_blocks(new_blocks).await;
            if new_state != *self.state_tx.borrow() {
                _ = self.state_tx.send_replace(new_state);
            }
        }
        // Return Ok to make the types align (see the method's doc comment why this is necessary).
        // Allow unreachable code to quiet warnings
        #[allow(unreachable_code)]
        Ok(())
    }
}
